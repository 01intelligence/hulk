use std::future::Future;
use std::io::ErrorKind;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures_util::ready;
use highway::{HighwayHash, Key};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, SeekFrom};

use crate::errors::StorageError;
use crate::io::AsyncReadFull;
use crate::storage::StorageApi;

const MAGIC_HIGHWAY_HASH_256_KEY: &[u8; 32] =
     b"\x4b\xe7\x34\xfa\x8e\x23\x8a\xcd\x26\x3e\x83\xe6\xbb\x96\x85\x52\x04\x0f\x93\x5d\xa3\x9f\x44\x14\x97\xe0\x9d\x13\x22\xde\x36\xa0";

fn high_way_hasher() -> highway::HighwayHasher {
    let key =
        unsafe { std::mem::transmute::<[u8; 32], [u64; 4]>(MAGIC_HIGHWAY_HASH_256_KEY.clone()) };
    highway::HighwayHasher::new(Key(key))
}

#[derive(Clone)]
pub struct HighwayHasher(highway::HighwayHasher, [u8; 32]);

impl super::BitrotHasher for HighwayHasher {
    fn append(&mut self, bytes: &[u8]) {
        self.0.append(bytes)
    }

    fn finish(&mut self) -> &[u8] {
        let hash = self.0.clone().finalize256();
        self.1 = unsafe { std::mem::transmute::<[u64; 4], [u8; 32]>(hash) };
        &self.1[..]
    }

    fn reset(&mut self) {
        self.0 = high_way_hasher();
    }
}

impl HighwayHasher {
    pub const fn output_size() -> usize {
        32
    }
}

impl Default for HighwayHasher {
    fn default() -> Self {
        Self(high_way_hasher(), [0u8; 32])
    }
}

#[pin_project::pin_project]
pub struct HighwayBitrotWriter {
    #[pin]
    writer: Box<dyn AsyncWrite + Unpin>,
    hasher: highway::HighwayHasher,
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
        storage: &StorageApi,
        volume: &str,
        path: &str,
        length: Option<u64>,
        shard_size: u64,
    ) -> anyhow::Result<Self> {
        let mut total_size = None;
        if let Some(length) = length {
            let bitrot_sums_size = crate::utils::ceil_frac(length, shard_size)
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

type BuildReaderInner<'a> = Pin<
    Box<dyn 'a + Future<Output = anyhow::Result<Box<dyn AsyncRead + Unpin + Send>>> + Send + Sync>,
>;
struct BuildReader<'a>(BuildReaderInner<'a>);
unsafe impl<'a> Send for BuildReader<'a> {}

pub struct HighwayBitrotReader<'a> {
    build_reader: Option<Box<dyn 'a + Send + Sync + FnOnce(u64) -> BuildReader<'a>>>,
    reader: Option<Box<dyn AsyncRead + Unpin + Send>>,
    hasher: highway::HighwayHasher,
    shard_size: u64,
    offset_cur: u64,
    hash: [u8; 32],
}

impl<'a> HighwayBitrotReader<'a> {
    pub async fn new<'b: 'a>(
        storage: &'b StorageApi,
        data: Vec<u8>,
        volume: String,
        path: String,
        till_offset: u64,
        shard_size: u64,
    ) -> HighwayBitrotReader<'a> {
        let till_offset = crate::utils::ceil_frac(till_offset, shard_size)
            * (std::mem::size_of::<[u64; 4]>() as u64)
            + till_offset;

        let build_reader = move |offset: u64| {
            let fut = async move {
                let stream_offset =
                    (offset / shard_size) * (std::mem::size_of::<[u64; 4]>() as u64) + offset;
                if data.is_empty() {
                    storage
                        .read_file_reader(
                            &volume,
                            &path,
                            stream_offset,
                            till_offset - stream_offset,
                        )
                        .await
                } else {
                    let mut reader = std::io::Cursor::new(data);
                    reader.seek(SeekFrom::Start(stream_offset)).await?;
                    let reader = reader.take(till_offset - stream_offset);
                    Ok(Box::new(reader) as Box<dyn AsyncRead + Unpin + Send>)
                }
            };
            BuildReader(Box::pin(fut) as BuildReaderInner)
        };
        Self {
            build_reader: Some(Box::new(build_reader)),
            reader: None,
            hasher: high_way_hasher(),
            shard_size,
            offset_cur: 0,
            hash: [0u8; 32],
        }
    }
}

unsafe impl<'a> Send for HighwayBitrotReader<'a> {}

#[async_trait]
impl<'a> crate::io::AsyncReadAt for HighwayBitrotReader<'a> {
    async fn read_at(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        assert_eq!(offset % self.shard_size, 0);
        let reader = match &mut self.reader {
            None => {
                let build_reader = self.build_reader.take().unwrap();
                let build_reader = build_reader(offset).0;
                self.reader = Some(
                    build_reader
                        .await
                        .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?,
                );
                self.offset_cur = offset;
                self.reader.as_mut().unwrap()
            }
            Some(reader) => reader,
        };

        assert_eq!(offset, self.offset_cur);

        let n = reader.read_full(&mut self.hash[..]).await?;
        if n < self.hash.len() {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        let n = reader.read_full(buf).await?;
        if n < self.hash.len() {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        let hasher = self.hasher.clone(); // TODO: avoid clone?
        let hash = hasher.hash256(buf);
        let hash = unsafe { std::mem::transmute::<[u64; 4], [u8; 32]>(hash) };
        if hash != self.hash {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                StorageError::FileCorrupt,
            ));
        }
        self.offset_cur += buf.len() as u64;
        Ok(n)
    }
}
