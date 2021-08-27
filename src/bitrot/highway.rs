use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::ready;
use highway::{HighwayHash, HighwayHasher, Key};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::storage::StorageApi;

const MAGIC_HIGHWAY_HASH_256_KEY: &[u8; 32] =
     b"\x4b\xe7\x34\xfa\x8e\x23\x8a\xcd\x26\x3e\x83\xe6\xbb\x96\x85\x52\x04\x0f\x93\x5d\xa3\x9f\x44\x14\x97\xe0\x9d\x13\x22\xde\x36\xa0";

fn high_way_hasher() -> HighwayHasher {
    let key =
        unsafe { std::mem::transmute::<[u8; 32], [u64; 4]>(MAGIC_HIGHWAY_HASH_256_KEY.clone()) };
    HighwayHasher::new(Key(key))
}

#[pin_project::pin_project]
pub struct HighwayBitrotWriter {
    #[pin]
    writer: Box<dyn AsyncWrite + Unpin>,
    hasher: HighwayHasher,
    state: State,
}

enum State {
    Start,
    PollHash([u8; 32], usize),
    PollContent(usize),
}

impl HighwayBitrotWriter {
    pub fn new(writer: Box<dyn AsyncWrite + Unpin>) -> Self {
        let hasher = high_way_hasher();
        Self {
            writer,
            hasher,
            state: State::Start,
        }
    }

    pub async fn new_with_storage_api(
        storage: StorageApi,
        volume: &str,
        path: &str,
        length: Option<u64>,
        shard_size: u64,
    ) -> anyhow::Result<Self> {
        let mut total_size = None;
        if let Some(length) = length {
            let bitrot_sums_size = (crate::utils::ceil_frac(length as i64, shard_size as i64)
                as u64)
                * (std::mem::size_of::<[u64; 4]>() as u64);
            total_size = Some(bitrot_sums_size + length);
        }
        let writer = storage.create_file_writer(volume, path, total_size).await?;
        let hasher = high_way_hasher();
        Ok(Self {
            writer,
            hasher,
            state: State::Start,
        })
    }
}

impl AsyncWrite for HighwayBitrotWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        loop {
            let this = self.as_mut().project();
            match this.state {
                State::Start => {
                    let hasher = this.hasher.clone(); // TODO: avoid clone?
                    let hash = hasher.hash256(buf);
                    let hash = unsafe { std::mem::transmute::<[u64; 4], [u8; 32]>(hash) };
                    *this.state = State::PollHash(hash, 0);
                }
                State::PollHash(ref hash, ref mut written) => {
                    let n = ready!(this.writer.poll_write(cx, &hash[*written..]))?;
                    *written += n;
                    if n == 0 {
                        // No longer able to write
                        return Poll::Ready(Ok(0));
                    } else if *written == hash.len() {
                        *this.state = State::PollContent(buf.len());
                    }
                }
                State::PollContent(ref mut remaining) => {
                    let n = ready!(this.writer.poll_write(cx, buf))?;
                    *remaining -= n;
                    if *remaining == 0 {
                        // Has written whole buf content.
                        *this.state = State::Start;
                    }
                    return Poll::Ready(Ok(n));
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().writer.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.project().writer.poll_shutdown(cx)
    }
}

#[pin_project::pin_project]
pub struct HighwayBitrotReader {
    #[pin]
    reader: Box<dyn AsyncRead + Unpin>,
    hasher: HighwayHasher,
    state: State,
}
