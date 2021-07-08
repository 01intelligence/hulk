use std::any::Any;
use std::pin::Pin;
use std::task::{Context, Poll};

use md5::Digest;
use tokio::io::{AsyncRead, ReadBuf};

use super::*;

pub trait Tagger {
    fn etag(&self) -> Option<ETag>;
}

pub trait MaybeTagger {
    fn as_tagger(&self) -> Option<&dyn Tagger>;
}

default impl<T: Tagger> MaybeTagger for T {
    fn as_tagger(&self) -> Option<&dyn Tagger> {
        Some(self)
    }
}

// impl<R: AsyncRead> MaybeTagger for R {
//     fn as_tagger(&self) -> Option<&dyn Tagger> {
//         None
//     }
// }

impl<R: AsyncRead + MaybeTagger> MaybeTagger for tokio::io::Take<R> {
    fn as_tagger(&self) -> Option<&dyn Tagger> {
        self.get_ref().as_tagger()
    }
}

pub struct WrapReader<R: AsyncRead> {
    reader: R,
}

impl<R: AsyncRead + Unpin> AsyncRead for WrapReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().reader).poll_read(cx, buf)
    }
}

impl<R: AsyncRead + MaybeTagger> Tagger for WrapReader<R> {
    fn etag(&self) -> Option<ETag> {
        self.reader.as_tagger().map(|t| t.etag()).flatten()
    }
}

impl<R: AsyncRead> WrapReader<R> {
    pub fn wrap(reader: R) -> WrapReader<R> {
        WrapReader { reader }
    }
}

pub struct Reader<R: AsyncRead> {
    src: R,
    md5: md5::Md5,
    checksum: Option<ETag>,
    read_n: usize,
}

impl<R: AsyncRead> Reader<R> {
    pub fn new(r: R, etag: Option<ETag>) -> Reader<R> {
        Reader {
            src: r,
            md5: md5::Md5::new(),
            checksum: etag,
            read_n: 0,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for Reader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let r = self.get_mut();
        let len_prev = buf.filled().len();
        let poll = Pin::new(&mut r.src).poll_read(cx, buf);
        let filled = buf.filled();
        r.read_n = filled.len() - len_prev;
        r.md5.update(&filled[len_prev..]);
        if poll.is_ready() && r.read_n == 0 && r.checksum.is_some() {
            let etag = r.etag().unwrap();
            if Some(etag) != r.checksum {
                return Poll::Ready(Err(std::io::ErrorKind::Other.into()));
            }
        }
        poll
    }
}

impl<R: AsyncRead> Tagger for Reader<R> {
    fn etag(&self) -> Option<ETag> {
        let r = self.md5.clone().finalize();
        Some(ETag(r.to_vec()))
    }
}
