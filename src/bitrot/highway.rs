use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::ready;
use highway::{HighwayHash, HighwayHasher, Key};
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};

const MAGIC_HIGHWAY_HASH_256_KEY: &[u8; 32] =
     b"\x4b\xe7\x34\xfa\x8e\x23\x8a\xcd\x26\x3e\x83\xe6\xbb\x96\x85\x52\x04\x0f\x93\x5d\xa3\x9f\x44\x14\x97\xe0\x9d\x13\x22\xde\x36\xa0";

#[pin_project::pin_project]
pub struct HighwayBitrotWriter<T: AsyncWrite + Unpin> {
    #[pin]
    writer: T,
    hasher: HighwayHasher,
}

fn high_way_hasher() -> HighwayHasher {
    let key =
        unsafe { std::mem::transmute::<[u8; 32], [u64; 4]>(MAGIC_HIGHWAY_HASH_256_KEY.clone()) };
    HighwayHasher::new(Key(key))
}

impl<T: AsyncWrite + Unpin> HighwayBitrotWriter<T> {
    pub fn new(writer: T, shard_size: u64) -> Self {
        let hasher = high_way_hasher();
        Self { writer, hasher }
    }

    pub fn new_with_storage_api() -> Self {
        todo!()
    }

    pub async fn write(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let hasher = high_way_hasher();
        let hash = hasher.hash256(buf);
        for h in hash {
            self.writer.write_u64_le(h).await?;
        }
        self.writer.write_all(buf).await
    }
}
