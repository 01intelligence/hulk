mod utils;

use std::any::Any;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::ensure;
use procfs::sys::fs::binfmt_misc::enabled;
use sha2::Digest;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
pub use utils::*;

use crate::etag::{self, ETag, MaybeTagger, Tagger};

pub trait ReadMaybeTagger<'a> = AsyncRead + MaybeTagger + Unpin + 'a;

pub struct Reader<'a, R: ReadMaybeTagger<'a>> {
    src: ReaderInner<'a, R>,
    bytes_read: usize,
    size: isize,
    actual_size: usize,
    checksum: ETag,
    content_sha256: Vec<u8>,
    sha256: Option<sha2::Sha256>,
    _phantom: PhantomData<&'a R>,
}

pub enum ReaderInner<'a, R: ReadMaybeTagger<'a>> {
    Reader(R),
    EtagReader(etag::Reader<R>),
    LimitedEtagReader(etag::Reader<tokio::io::Take<R>>),
    LimitedEtagWrapReader(etag::WrapReader<tokio::io::Take<R>>),
    LimitedEtagWrapInnerReader(etag::WrapReader<tokio::io::Take<Box<ReaderInner<'a, R>>>>),
    _Phantom(PhantomData<&'a R>),
}

impl<'a, R: ReadMaybeTagger<'a>> AsyncRead for ReaderInner<'a, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match *self.get_mut() {
            ReaderInner::Reader(ref mut r) => Pin::new(r).poll_read(cx, buf),
            ReaderInner::EtagReader(ref mut r) => Pin::new(r).poll_read(cx, buf),
            ReaderInner::LimitedEtagReader(ref mut r) => Pin::new(r).poll_read(cx, buf),
            ReaderInner::LimitedEtagWrapReader(ref mut r) => Pin::new(r).poll_read(cx, buf),
            ReaderInner::LimitedEtagWrapInnerReader(ref mut r) => Pin::new(r).poll_read(cx, buf),
            _ => panic!(),
        }
    }
}

impl<'a, R: ReadMaybeTagger<'a>> MaybeTagger for ReaderInner<'a, R> {
    fn as_tagger(&self) -> Option<&dyn Tagger> {
        match *self {
            ReaderInner::Reader(ref r) => r.as_tagger(),
            ReaderInner::EtagReader(ref r) => r.as_tagger(),
            ReaderInner::LimitedEtagReader(ref r) => r.as_tagger(),
            ReaderInner::LimitedEtagWrapReader(ref r) => r.as_tagger(),
            ReaderInner::LimitedEtagWrapInnerReader(ref r) => r.as_tagger(),
            _ => panic!(),
        }
    }
}

impl<'a, R: ReadMaybeTagger<'a>> MaybeTagger for Box<ReaderInner<'a, R>> {
    fn as_tagger(&self) -> Option<&dyn Tagger> {
        (**self).as_tagger()
    }
}

impl<'a, R: ReadMaybeTagger<'a>> AsyncRead for Reader<'a, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let r = self.get_mut();
        let len_prev = buf.filled().len();
        let poll = Pin::new(&mut r.src).poll_read(cx, buf);
        match poll {
            Poll::Pending => {
                return poll;
            }
            Poll::Ready(Err(err)) => {
                // TODO: etag VerifyError
                return Poll::Ready(Err(err));
            }
            Poll::Ready(Ok(_)) => {}
        }
        let filled = buf.filled();
        let n = filled.len() - len_prev;
        r.bytes_read += n;
        if let Some(ref mut sha256) = r.sha256 {
            sha256.update(&filled[len_prev..]);
            if n == 0 {
                let sha256 = sha256.clone().finalize().to_vec();
                if r.content_sha256 != sha256 {
                    return Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        Error::Sha256Mismatch {
                            expected_sha256: hex::encode(&r.content_sha256),
                            calculated_sha256: hex::encode(sha256),
                        },
                    )));
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl<'a, R: ReadMaybeTagger<'a>> MaybeTagger for Reader<'a, R> {
    fn as_tagger(&self) -> Option<&dyn Tagger> {
        self.src.as_tagger()
    }
}

impl<'a, R: ReadMaybeTagger<'a>> Reader<'a, R> {
    pub fn from_reader(
        mut src: Reader<'a, R>,
        size: isize,
        md5_hex: &str,
        sha256_hex: &str,
        actual_size: usize,
    ) -> anyhow::Result<Reader<'a, R>> {
        let md5 = hex::decode(md5_hex)?;
        let sha256 = hex::decode(sha256_hex)?;

        ensure!(src.bytes_read == 0, "hash: already read from hash reader");
        ensure!(
            (&src.checksum).is_empty() || md5.is_empty() || src.checksum == ETag::new(md5.clone()),
            Error::BadDigest {
                expected_md5: src.checksum.to_string(),
                calculated_md5: md5_hex.to_owned(),
            }
        );
        ensure!(
            src.content_sha256.is_empty() || sha256.is_empty() || src.content_sha256 == sha256,
            Error::Sha256Mismatch {
                expected_sha256: hex::encode(src.content_sha256),
                calculated_sha256: sha256_hex.to_owned(),
            }
        );
        ensure!(
            src.size < 0 || size < 0 || src.size == size,
            Error::SizeMismatch {
                want: src.size,
                got: size,
            }
        );
        src.checksum = ETag::new(md5);
        src.content_sha256 = sha256;
        if src.size < 0 && size >= 0 {
            src.src = ReaderInner::LimitedEtagWrapInnerReader(etag::WrapReader::wrap(
                Box::new(src.src).take(size as u64),
            ));
            src.size = size;
        }
        if src.actual_size <= 0 && actual_size >= 0 {
            src.actual_size = actual_size;
        }
        return Ok(src);
    }

    pub fn new(
        mut src: R,
        size: isize,
        md5_hex: &str,
        sha256_hex: &str,
        actual_size: usize,
    ) -> anyhow::Result<Reader<'a, R>> {
        let md5 = hex::decode(md5_hex)?;
        let sha256 = hex::decode(sha256_hex)?;

        let s: ReaderInner<R>;
        if size >= 0 {
            if src.as_tagger().is_some() {
                s = ReaderInner::LimitedEtagWrapReader(etag::WrapReader::wrap(
                    src.take(size as u64),
                ));
            } else {
                s = ReaderInner::LimitedEtagReader(etag::Reader::new(
                    src.take(size as u64),
                    Some(ETag::new(md5)),
                ));
            }
        } else if src.as_tagger().is_none() {
            s = ReaderInner::EtagReader(etag::Reader::new(src, Some(ETag::new(md5))));
        } else {
            s = ReaderInner::Reader(src);
        }

        let hash = if !sha256.is_empty() {
            Some(sha2::Sha256::new())
        } else {
            None
        };

        Ok(Reader {
            src: s,
            bytes_read: 0,
            size,
            actual_size,
            checksum: ETag::default(),
            content_sha256: sha256,
            sha256: hash,
            _phantom: PhantomData,
        })
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(
        "Bad sha256: Expected {expected_sha256} does not match calculated {calculated_sha256}"
    )]
    Sha256Mismatch {
        expected_sha256: String,
        calculated_sha256: String,
    },
    #[error("Bad digest: Expected {expected_md5} does not match calculated {calculated_md5}")]
    BadDigest {
        expected_md5: String,
        calculated_md5: String,
    },
    #[error("Size mismatch: got {want}, want {got}")]
    SizeMismatch { want: isize, got: isize },
}
