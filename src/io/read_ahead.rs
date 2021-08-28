use std::cell::RefCell;
use std::future::Future;
use std::io::ErrorKind;

use futures_util::ready;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;

use crate::prelude::*;

pub struct ReadAhead {
    inner: Arc<RefCell<Inner>>,
    task_handle: Option<JoinHandle<()>>,
    state: State,
}

struct Inner {
    ready: Receiver<Buf>,
    reuse: Option<Sender<Buf>>,
    cur: Option<Buf>,
}

enum State {
    Idle,
    Fill(Option<Pin<Box<dyn Future<Output = std::io::Result<()>>>>>),
}

#[derive(Debug)]
struct Buf {
    buf: Vec<u8>,
    offset: usize,
    size: usize,
    err: Option<std::io::Error>,
}

impl Buf {
    fn is_empty(&self) -> bool {
        self.offset >= self.size
    }

    async fn read<R: AsyncRead + Unpin>(&mut self, r: &mut R) -> bool {
        match r.read(&mut self.buf[..]).await {
            Ok(n) => {
                self.offset = 0;
                self.size = n;
                true
            }
            Err(err) => {
                self.err = Some(err);
                false
            }
        }
    }

    fn advance(&mut self, n: usize) {
        self.offset += n;
    }

    fn buf(&self) -> &[u8] {
        &self.buf[self.offset..self.size]
    }
}

impl Drop for ReadAhead {
    fn drop(&mut self) {
        if let Some(task_handle) = self.task_handle.take() {
            let mut inner = (*self.inner).borrow_mut();
            drop(inner.reuse.take());
            inner.ready.close();
            tokio::task::block_in_place(move || {
                tokio::runtime::Handle::current().block_on(async move {
                    task_handle.await;
                })
            });
        }
    }
}

impl ReadAhead {
    pub async fn new<R: AsyncRead + Unpin + Send + 'static>(
        r: R,
        buffers: usize,
        buffer_size: usize,
    ) -> Self {
        debug_assert!(buffers > 0 && buffer_size > 0);

        let (ready_tx, ready_rx) = channel(buffers);
        let (reuse_tx, mut reuse_rx) = channel(buffers);

        // Create buffers.
        for _ in 0..buffers {
            reuse_tx
                .send(Buf {
                    buf: vec![0u8; buffer_size],
                    err: None,
                    offset: 0,
                    size: 0,
                })
                .await
                .unwrap();
        }

        // Run async reader.
        let task_handle = tokio::spawn(async move {
            let mut r = r;
            while let Some(mut buf) = reuse_rx.recv().await {
                let success = buf.read(&mut r).await;
                if let Err(_) = ready_tx.send(buf).await {
                    return;
                }
                if !success {
                    return;
                }
            }
        });

        ReadAhead {
            inner: Arc::new(RefCell::new(Inner {
                ready: ready_rx,
                reuse: Some(reuse_tx),
                cur: None,
            })),
            task_handle: Some(task_handle),
            state: State::Idle,
        }
    }

    async fn fill(inner: Arc<RefCell<Inner>>) -> std::io::Result<()> {
        let mut inner = (*inner).borrow_mut();
        if inner.cur.is_none() || inner.cur.as_ref().unwrap().is_empty() {
            if let Some(buf) = inner.cur.take() {
                if let Err(_) = inner.reuse.as_ref().unwrap().send(buf).await {
                    return Err(std::io::Error::new(
                        ErrorKind::Other,
                        "read-ahead task gone",
                    ));
                }
            }
            if let Some(buf) = inner.ready.recv().await {
                inner.cur = Some(buf);
            } else {
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    "read-ahead task gone",
                ));
            }
        }
        Ok(())
    }
}

impl AsyncRead for ReadAhead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                State::Idle => {
                    // Swap buffer
                    this.state = State::Fill(Some(Box::pin(Self::fill(Arc::clone(&this.inner)))));
                }
                State::Fill(fill) => {
                    ready!(fill.as_mut().unwrap().as_mut().poll(cx))?;

                    let mut inner = (*this.inner).borrow_mut();

                    // Give read
                    let cur = inner.cur.as_mut().unwrap();
                    let bytes = cur.buf();
                    let n = buf.remaining().min(bytes.len());
                    buf.put_slice(&bytes[..n]);
                    cur.advance(n);

                    if cur.is_empty() {
                        // Return any error.
                        if let Some(err) = cur.err.take() {
                            return Poll::Ready(Err(err));
                        }
                    }

                    this.state = State::Idle;
                    return Poll::Ready(Ok(()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_read_ahead() {
        let src = b"testbuffer";
        let mut reader = ReadAhead::new(&src[..], 4, 10000).await;

        let mut buf = [0u8; 100];
        let n = reader.read(&mut buf[..]).await.unwrap();
        assert_eq!(n, src.len());
        assert_eq!(&buf[..n], &src[..]);

        // Test EOF.
        let n = reader.read(&mut buf[..]).await.unwrap();
        assert_eq!(n, 0);

        // Test again after EOF.
        let n = reader.read(&mut buf[..]).await.unwrap();
        assert_eq!(n, 0);

        drop(reader);
    }
}
