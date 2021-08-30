mod highway;

use std::io::Error;

use futures_util::ready;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

pub use self::highway::*;
use crate::errors::StorageError;
use crate::io::AsyncReadFull;
use crate::prelude::{Context, Pin, Poll};

pub fn bitrot_self_test() {}

pub const DEFAULT_BITROT_ALGORITHM: BitrotAlgorithm = BitrotAlgorithm::HighwayHash256;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum BitrotAlgorithm {
    HighwayHash256,
}

pub trait BitrotHasher {
    fn write(&mut self, bytes: &[u8]);
    fn finish(&mut self) -> &[u8];
    fn reset(&mut self);
}

impl AsyncWrite for Box<dyn BitrotHasher + Unpin> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.get_mut().write(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl BitrotAlgorithm {
    pub fn hasher(&self) -> Box<dyn BitrotHasher + Unpin> {
        Box::new(match self {
            BitrotAlgorithm::HighwayHash256 => HighwayHasher::default(),
        })
    }

    pub const fn output_size(&self) -> usize {
        match self {
            BitrotAlgorithm::HighwayHash256 => HighwayHasher::output_size(),
        }
    }
}

pub struct BitrotVerifier {
    pub algorithm: BitrotAlgorithm,
    pub hash: [u8; 32],
}

pub async fn bitrot_verify<R: AsyncRead + Unpin>(
    mut reader: R,
    want_size: u64,
    part_size: u64,
    algo: BitrotAlgorithm,
    _want: &[u8],
    mut shard_size: u64,
) -> anyhow::Result<()> {
    let mut reader = Some(reader);

    // Calculate the size of the bitrot file and compare
    // it with the actual file size.
    if want_size
        != crate::utils::ceil_frac(part_size, shard_size) * (algo.output_size() as u64) + part_size
    {
        return Err(StorageError::FileCorrupt.into());
    }

    let mut hasher = algo.hasher();
    let mut hash_buf = vec![0u8; algo.output_size()];
    let mut buf_guard = Some(crate::xl_storage::XL_POOL_SMALL.get().await?);

    let mut left = want_size;
    while left > 0 {
        hasher.reset();
        let n = reader.as_mut().unwrap().read_full(&mut hash_buf).await?;

        assert!(left >= n as u64);
        left -= n as u64;
        if left < shard_size {
            shard_size = left;
        }

        let mut buf_reader = crate::io::BufReader::new(
            reader.take().unwrap().take(shard_size),
            buf_guard.take().unwrap(),
        );
        let n = tokio::io::copy_buf(&mut buf_reader, &mut hasher).await?;
        left -= n;

        let (buf_reader, g) = buf_reader.into_inner();
        reader.insert(buf_reader.into_inner());
        buf_guard.insert(g);

        if hasher.finish() != &hash_buf {
            return Err(StorageError::FileCorrupt.into());
        }
    }

    Ok(())
}
