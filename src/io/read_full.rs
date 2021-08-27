use std::future::Future;
use std::io;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::ready;
use pin_project::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

pub trait AsyncReadFull: AsyncRead {
    fn read_full<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFull<'a, Self>
    where
        Self: Unpin,
    {
        read_full(self, buf)
    }
}

impl<R: AsyncRead + ?Sized> AsyncReadFull for R {}

fn read_full<'a, A>(reader: &'a mut A, buf: &'a mut [u8]) -> ReadFull<'a, A>
where
    A: AsyncRead + Unpin + ?Sized,
{
    ReadFull {
        reader,
        buf: ReadBuf::new(buf),
        _pin: PhantomPinned,
    }
}

#[pin_project]
#[derive(Debug)]
pub struct ReadFull<'a, A: ?Sized> {
    reader: &'a mut A,
    buf: ReadBuf<'a>,
    // Make this future `!Unpin` for compatibility with async trait methods.
    #[pin]
    _pin: PhantomPinned,
}

impl<A> Future for ReadFull<'_, A>
where
    A: AsyncRead + Unpin + ?Sized,
{
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        let mut me = self.project();

        loop {
            let rem = me.buf.remaining();
            if rem != 0 {
                ready!(Pin::new(&mut *me.reader).poll_read(cx, &mut me.buf))?;
                if me.buf.remaining() == rem {
                    // EOF
                    return Poll::Ready(Ok(me.buf.filled().len()));
                }
            } else {
                return Poll::Ready(Ok(me.buf.capacity()));
            }
        }
    }
}
