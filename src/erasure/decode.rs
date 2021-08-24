use std::io::ErrorKind;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWrite};

use super::*;
use crate::errors::{AsError, ReducibleError, StorageError};
use crate::utils::AsyncReadExt;

struct ParallelReader<'a> {
    readers: Vec<(usize, &'a mut Option<Box<dyn AsyncReadAt + Send + Unpin>>)>,
    data_blocks: usize,
    offset: u64,
    shard_size: u64,
    shard_file_size: u64,
    buf: Vec<Option<Vec<u8>>>, // TODO: cache pool?
}

impl<'a> ParallelReader<'a> {
    fn new(
        readers: &'a mut [Option<Box<dyn AsyncReadAt + Send + Unpin>>],
        erasure: &Erasure,
        offset: u64,
        total_length: u64,
    ) -> Self {
        let size = readers.len();
        Self {
            readers: readers.iter_mut().enumerate().collect(),
            data_blocks: erasure.data_blocks,
            offset,
            shard_size: erasure.shard_size() as u64,
            shard_file_size: erasure.shard_file_size(total_length) as u64,
            buf: vec![None; size],
        }
    }

    /// Reorder readers as preferred chosen first.
    fn prefer_readers(&mut self, prefer: &[bool]) {
        assert_eq!(prefer.len(), self.readers.len());
        use std::cmp::Ordering;
        self.readers.sort_by(|a, b| {
            if b.1.is_none() || !prefer[b.0] {
                return Ordering::Less;
            }
            if a.1.is_none() || !prefer[a.0] {
                return Ordering::Greater;
            }
            Ordering::Equal
        })
    }

    async fn read(
        &mut self,
    ) -> anyhow::Result<(&mut Vec<Option<Vec<u8>>>, Option<std::io::Error>)> {
        for b in &mut self.buf {
            if let Some(b) = b {
                b.clear(); // clear but retain space
            }
        }

        if self.offset + self.shard_size > self.shard_file_size {
            self.shard_size = self.shard_file_size - self.offset
        }
        if self.shard_size == 0 {
            return Ok((&mut self.buf, None)); // no need to read
        }
        let offset = self.offset;
        let shard_size = self.shard_size as usize;

        let (read_trigger_tx, mut read_trigger_rx) = tokio::sync::mpsc::channel(self.readers.len());
        for _ in 0..self.data_blocks {
            // Setup read triggers for `data_blocks` number of reads so that it reads in parallel.
            read_trigger_tx.send(true).await;
        }

        let mut reader_iter = 0;
        let mut handles = Vec::new();
        let mut success_count = Arc::new(AtomicUsize::new(0));

        while let Some(read_trigger) = read_trigger_rx.recv().await {
            if success_count.load(Ordering::SeqCst) >= self.data_blocks {
                break; // can decode now
            }

            if reader_iter == self.readers.len() {
                break; // oops, no remaining reader to read
            }

            if !read_trigger {
                continue;
            }

            let reader_index = reader_iter;
            reader_iter += 1; // for next iter

            let read_trigger_tx = read_trigger_tx.clone();
            let (buf_idx, ref mut reader) = self.readers[reader_index];
            if reader.is_none() {
                // Since reader is none, trigger another read.
                read_trigger_tx.send(true).await;
                continue;
            }

            let buf = self.buf[buf_idx].get_or_insert_default();
            if buf.is_empty() {
                // Reading first time on this disk, hence buf needs to be allocated.
                // Subsequent reads will reuse this buf.
                buf.resize(shard_size, 0u8);
            } else if buf.len() > shard_size {
                // For the last shard, the shard size might be less than previous shard sizes.
                // Hence truncate buf to the right size.
                buf.truncate(shard_size);
            }

            // Safety: this buf will only be accessed by this task.
            let buf_ptr = RawPtr(unsafe { buf as *mut Vec<u8> });
            // Safety: this reader will only be accessed by this task.
            let reader = RawPtr(unsafe {
                reader.as_mut().unwrap() as *mut Box<dyn AsyncReadAt + Send + Unpin>
            });
            let success_count = Arc::clone(&success_count);

            handles.push(tokio::spawn(async move {
                let buf = unsafe { buf_ptr.0.as_mut().unwrap() };
                let reader = unsafe { reader.0.as_mut().unwrap() };
                match reader.read_at(&mut buf[..], offset).await {
                    Err(e) => {
                        let mut err = None;
                        if let Some(eref) = e.as_error::<StorageError>() {
                            if eref == &StorageError::FileNotFound
                                || eref == &StorageError::FileCorrupt
                            {
                                err = Some(e);
                            }
                        }

                        // Since read failed, trigger another read.
                        read_trigger_tx.send(true).await;
                        (Some(reader_index), err)
                    }
                    Ok(n) => {
                        if n < shard_size {
                            read_trigger_tx.send(true).await;
                            let err =
                                std::io::Error::new(ErrorKind::Other, StorageError::FileNotFound);
                            return (Some(reader_index), Some(err));
                        }
                        success_count.fetch_add(1, Ordering::SeqCst);
                        // Since this read succeed, no need to trigger another immediate read.
                        // But trigger next try to check whether can decode.
                        read_trigger_tx.send(false).await;
                        (None, None)
                    }
                }
            }))
        }

        let mut err = None;
        for r in futures_util::future::join_all(handles).await {
            let r = r.unwrap(); // no task should panic
            if let Some(idx) = r.0 {
                *self.readers[idx].1 = None; // notify reader fault to caller
            }
            if err.is_none() && r.1.is_some() {
                err = r.1;
            }
        }

        if success_count.load(Ordering::SeqCst) >= self.data_blocks {
            self.offset += self.shard_size;

            return Ok((&mut self.buf, err));
        }

        // Cannot decode, just return error.
        Err(StorageError::ErasureReadQuorum.into())
    }
}

struct RawPtr<T>(*mut T);

unsafe impl<T> Send for RawPtr<T> {}

#[async_trait]
pub trait AsyncReadAt {
    async fn read_at(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<usize>;
}

#[async_trait]
impl<T: AsyncRead + AsyncSeek + Send + Unpin> AsyncReadAt for T {
    async fn read_at(&mut self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        let _ = self.seek(std::io::SeekFrom::Start(offset)).await?;
        self.read_full(buf).await
    }
}

impl Erasure {
    pub async fn decode(
        &self,
        writer: &mut (impl AsyncWrite + Unpin),
        readers: &mut [Option<Box<dyn AsyncReadAt + Send + Unpin>>],
        offset: u64,
        length: u64,
        total_length: u64,
        prefer: &[bool],
    ) -> anyhow::Result<(u64, Option<std::io::Error>)> {
        assert!(offset + length <= total_length);
        if length == 0 {
            return Ok((0, None));
        }

        let readers_len = readers.len();
        let mut reader = ParallelReader::new(readers, self, offset, total_length);
        if prefer.len() == readers_len {
            reader.prefer_readers(prefer);
        }

        let block_size = self.block_size as u64;
        let start_block = offset / block_size;
        let end_block = (offset + length) / block_size;

        let mut bytes_written = 0u64;
        let mut err = None;
        for block in start_block..=end_block {
            let (block_offset, block_length) = if start_block == end_block {
                (offset % block_size, length)
            } else if block == start_block {
                (offset % block_size, block_size - (offset % block_size))
            } else if block == end_block {
                (0, (offset + length) % block_size)
            } else {
                (0, block_size)
            };
            if block_length == 0 {
                break;
            }

            let (buf, rerr) = reader.read().await?;
            // Though there are enough data for reconstruction, set error for caller further healing.
            if err.is_none() && rerr.is_some() {
                err = rerr;
            }

            self.decode_data_blocks(buf)?;

            let n = write_data_blocks(writer, buf, self.data_blocks, block_offset, block_length)
                .await?;
            bytes_written += n;
        }

        if bytes_written != length {
            return Err(StorageError::LessData.into());
        }

        Ok((bytes_written, err))
    }
}

async fn write_data_blocks(
    writer: &mut (impl AsyncWrite + Unpin),
    blocks: &Vec<Option<Vec<u8>>>,
    data_blocks: usize,
    mut offset: u64,
    length: u64,
) -> anyhow::Result<u64> {
    assert!(blocks.len() >= data_blocks);
    let blocks = &blocks[..data_blocks];

    let size = blocks.iter().fold(0, |acc, b| {
        if let Some(b) = b {
            return acc + b.len();
        }
        acc
    });
    if size < length as usize {
        use reed_solomon_erasure::Error::*;
        return Err(TooFewDataShards.into());
    }

    let mut write = length;

    let mut total_written = 0;

    for block in blocks {
        assert!(block.is_some());
        let mut block = &block.as_ref().unwrap()[..];
        let block_len = block.len() as u64;
        if offset >= block_len {
            offset -= block_len;
            continue;
        } else {
            block = &block[offset as usize..];
            offset = 0;
        }

        if write < block_len {
            let n = tokio::io::copy(&mut &block[..write as usize], writer).await?;
            total_written += n;
            break;
        }

        let n = tokio::io::copy(&mut block, writer).await?;
        write -= n;
        total_written += n;
    }

    Ok(total_written)
}
