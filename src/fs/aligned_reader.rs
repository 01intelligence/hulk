use std::future::Future;

use futures_util::ready;
use tokio::io::{AsyncRead, ReadBuf};

use super::*;
use crate::prelude::*;
use crate::uninterruptibly;
use crate::utils::{BufGuardMut, SendRawPtr};

#[pin_project::pin_project]
pub struct AlignedReader<G: BufGuardMut> {
    std: Arc<std::fs::File>,
    state: State,
    buf: &'static mut [u8],
    #[pin]
    buf_guard: G,
}

enum State {
    Idle(usize, usize),
    Busy(Option<tokio::task::JoinHandle<std::io::Result<usize>>>),
}

impl<G: BufGuardMut> AlignedReader<G> {
    /// Read using aligned buffer.
    ///
    /// Note that [`aligned_buf_guard`] must give buffer which is aligned to [`DIRECTIO_ALIGN_SIZE`] page boundaries.
    /// File [`f`] must be opened with DIRECT I/O enabled.
    pub fn new(f: std::fs::File, mut aligned_buf_guard: G) -> Self {
        let buf = aligned_buf_guard.buf_mut();
        assert_eq!(buf.len() % DIRECTIO_ALIGN_SIZE, 0);
        // Safety: lifetime of `buf` depends on `buf_guard`.
        let buf = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len()) };
        AlignedReader {
            std: Arc::new(f),
            state: State::Idle(0, 0),
            buf,
            buf_guard: aligned_buf_guard,
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if let State::Busy(rx) = self {
            let rx = rx.take().unwrap();
            tokio::task::block_in_place(move || {
                tokio::runtime::Handle::current().block_on(async move {
                    let _ = rx.await;
                })
            });
        }
    }
}

impl<G: BufGuardMut> AsyncRead for AlignedReader<G> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.project();

        loop {
            match *this.state {
                State::Idle(n, ref mut pos) => {
                    if n > 0 {
                        let size = buf.remaining().min(n - *pos);
                        buf.put_slice(&this.buf[*pos..(*pos + size)]);
                        *pos += size;
                        if buf.remaining() == 0 {
                            return Poll::Ready(Ok(()));
                        }
                    }

                    let std = this.std.clone();
                    let buf_ptr = SendRawPtr::new(this.buf.as_mut_ptr());
                    let size = this.buf.len();
                    let rx = tokio::task::spawn_blocking(move || {
                        let mut std = &mut &*std;
                        // Safety: validity of `buf` is guarantee by `buf_guard`.
                        let buf = unsafe { std::slice::from_raw_parts_mut(buf_ptr.to(), size) };
                        match uninterruptibly!(std.read(buf)) {
                            Ok(n) => Ok(n),
                            Err(err) => {
                                if err_invalid_arg(&err) {
                                    std.disable_direct_io()?;
                                    uninterruptibly!(std.read(buf)) // retry
                                } else {
                                    Err(err)
                                }
                            }
                        }
                    });
                    *this.state = State::Busy(Some(rx));
                }
                State::Busy(ref mut rx) => {
                    let n = ready!(Pin::new(rx.as_mut().unwrap()).poll(cx))??;
                    *this.state = State::Idle(n, 0);
                    if n == 0 {
                        return Poll::Ready(Ok(())); // eof
                    }
                }
            }
        }
    }
}
