use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

struct Bytes {
    ptr: *const u8,
    len: usize,
    advanced: Option<usize>,
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Bytes {
    #[inline]
    fn new() -> Bytes {
        const EMPTY: &[u8] = &[];
        Bytes {
            ptr: EMPTY.as_ptr(),
            len: EMPTY.len(),
            advanced: None,
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn assign(&mut self, buf: &[u8]) {
        self.ptr = buf.as_ptr();
        self.len = buf.len();
        self.advanced = None;
    }

    #[inline]
    fn advance(&mut self, cnt: usize) {
        assert!(
            cnt <= self.len,
            "cannot advance past `remaining`: {:?} <= {:?}",
            cnt,
            self.len,
        );

        unsafe {
            self.len -= cnt;
            self.ptr = self.ptr.offset(cnt as isize);
        }

        *self.advanced.get_or_insert_default() += cnt;
    }
}

struct Pipe {
    buffer: Bytes,
    write_waker: Option<Waker>,
    read_waker: Option<Waker>,
    is_closed: bool,
    rerr: Option<std::io::Error>,
    werr: Option<std::io::Error>,
}

pub struct PipeReader {
    read: Arc<Mutex<Pipe>>,
}

pub struct PipeWriter {
    write: Arc<Mutex<Pipe>>,
}

/// Creates a asynchronous in-memory pipe.
/// It can be used to connect code expecting an `AsyncRead`
/// with code expecting an `AsyncWrite`.
///
/// Reads and Writes on the pipe are matched one to one.
/// The data is copied directly from the Writer to the corresponding
/// Reader; there is no internal buffering.
///
/// Safety: unsafe!
pub fn pipe() -> (PipeWriter, PipeReader) {
    let pipe = Arc::new(Mutex::new(Pipe {
        buffer: Bytes::new(),
        write_waker: None,
        read_waker: None,
        is_closed: false,
        rerr: None,
        werr: None,
    }));
    (
        PipeWriter {
            write: pipe.clone(),
        },
        PipeReader { read: pipe },
    )
}

impl AsyncRead for PipeReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.read.lock().unwrap()).poll_read(cx, buf)
    }
}

impl AsyncWrite for PipeWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut *self.write.lock().unwrap()).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.write.lock().unwrap()).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.write.lock().unwrap()).poll_shutdown(cx)
    }
}

impl PipeReader {
    pub fn close_with_error(&mut self, err: std::io::Error) {
        self.read.lock().unwrap().close_read(Some(err));
    }
}

impl PipeWriter {
    pub fn close_with_error(&mut self, err: std::io::Error) {
        self.write.lock().unwrap().close_write(Some(err));
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        // notify the other side of the closure
        self.read.lock().unwrap().close_read(None);
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        // notify the other side of the closure
        self.write.lock().unwrap().close_write(None);
    }
}

impl Pipe {
    fn close_write(&mut self, werr: Option<std::io::Error>) {
        if self.werr.is_some() {
            return;
        }
        self.is_closed = true;
        self.werr = werr;
        // needs to notify any readers that no more data will come
        if let Some(waker) = self.read_waker.take() {
            waker.wake();
        }
    }

    fn close_read(&mut self, rerr: Option<std::io::Error>) {
        if self.rerr.is_some() {
            return;
        }
        self.is_closed = true;
        self.rerr = Some(rerr.unwrap_or_else(|| std::io::ErrorKind::BrokenPipe.into()));
        // needs to notify any writers that they have to abort
        if let Some(waker) = self.write_waker.take() {
            waker.wake();
        }
    }

    fn write_close_error(&mut self) -> std::io::Result<usize> {
        if self.werr.is_none() {
            if let Some(rerr) = self.rerr.replace(std::io::ErrorKind::Other.into()) {
                return Err(rerr);
            }
        }
        Err(std::io::ErrorKind::BrokenPipe.into())
    }

    fn read_close_error(&mut self) -> std::io::Result<()> {
        if self.rerr.is_none() {
            return if let Some(werr) = self.werr.replace(std::io::ErrorKind::Other.into()) {
                Err(werr)
            } else {
                Ok(()) // EOF
            };
        }
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}

impl AsyncRead for Pipe {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.is_closed {
            return Poll::Ready(self.read_close_error());
        }

        // If has buffer and has not been read.
        if !self.buffer.is_empty() && self.buffer.advanced.is_none() {
            let max = self.buffer.len.min(buf.remaining());
            if max > 0 {
                buf.put_slice(&self.buffer.as_ref()[..max]);
                self.buffer.advance(max);
                // The passed `buf` might have been empty, don't wake up if
                // no bytes have been moved.
                if let Some(waker) = self.write_waker.take() {
                    waker.wake();
                }
            }
            return Poll::Ready(Ok(()));
        }

        self.read_waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl AsyncWrite for Pipe {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.is_closed {
            return Poll::Ready(self.write_close_error());
        }

        // If has been written last time.
        if !self.buffer.is_empty() {
            return match self.buffer.advanced {
                // Has not been read
                None => {
                    self.write_waker = Some(cx.waker().clone());
                    Poll::Pending
                }
                // Has been read
                Some(advanced) => Poll::Ready(Ok(advanced)),
            };
        }

        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        self.buffer.assign(buf);
        if let Some(waker) = self.read_waker.take() {
            waker.wake();
        }
        self.write_waker = Some(cx.waker().clone());
        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.close_write(None);
        Poll::Ready(Ok(()))
    }
}
