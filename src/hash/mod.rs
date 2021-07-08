use tokio::io::{AsyncRead, AsyncReadExt};

use crate::etag;
use crate::etag::MaybeTagger;

pub struct Reader {
    src: Box<dyn AsyncRead>,
    bytes_read: usize,
    size: isize,
    actual_size: usize,
    checksum: etag::ETag,
    content_sha256: Vec<u8>,
}

impl Reader {
    pub fn new<R: AsyncRead + MaybeTagger + Unpin + 'static>(
        src: R,
        size: isize,
        md5_hex: &str,
        sha256_hex: &str,
        actual_size: usize,
    ) -> anyhow::Result<Reader> {
        let md5 = hex::decode(md5_hex)?;
        let sha256 = hex::decode(sha256_hex)?;

        let s: Box<dyn AsyncRead>;
        if size >= 0 {
            if src.as_tagger().is_some() {
                s = Box::new(etag::WrapReader::wrap(src.take(size as u64)));
            } else {
                s = Box::new(etag::Reader::new(
                    src.take(size as u64),
                    Some(etag::ETag(md5)),
                ));
            }
        } else if src.as_tagger().is_none() {
            s = Box::new(etag::Reader::new(src, Some(etag::ETag(md5))));
        } else {
            s = Box::new(src);
        }

        Ok(Reader {
            src: s,
            bytes_read: 0,
            size,
            actual_size,
            checksum: etag::ETag(Default::default()),
            content_sha256: sha256,
        })
    }
}
