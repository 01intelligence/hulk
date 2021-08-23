use std::io::Error;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWrite};

use super::*;
use crate::errors::{AsError, ReducibleError, StorageError};
use crate::utils::AsyncReadExt;

struct ParallelReader<'a> {
    readers: Vec<(usize, &'a mut Option<Box<dyn AsyncReadAt + Send + Unpin>>)>,
    data_blocks: usize,
    offset: u64,
    shard_size: usize,
    shard_file_size: usize,
    buf: Vec<Option<Vec<u8>>>,
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
            shard_size: erasure.shard_size(),
            shard_file_size: erasure.shard_file_size(total_length),
            buf: vec![None; size],
        }
    }

    fn prefer_readers(&mut self, prefer: &[bool]) {
        assert_eq!(prefer.len(), self.readers.len());
        use std::cmp::Ordering;
        self.readers.sort_by(|a, b| {
            if a.1.is_none() || !prefer[a.0] {
                return Ordering::Greater;
            }
            if b.1.is_none() || !prefer[b.0] {
                return Ordering::Less;
            }
            Ordering::Equal
        })
    }

    fn can_decode(&self, buf: &[Option<Vec<u8>>]) -> bool {
        buf.iter().fold(0, |acc, b| {
            if let Some(b) = b {
                if !b.is_empty() {
                    return acc + 1;
                }
            }
            acc
        }) >= self.data_blocks
    }

    async fn read(&mut self) -> anyhow::Result<&mut Vec<Option<Vec<u8>>>> {
        let (read_trigger_tx, mut read_trigger_rx) = tokio::sync::mpsc::channel(self.readers.len());
        for _ in 0..self.data_blocks {
            read_trigger_tx.send(true).await;
        }
        let mut reader_index = 0;
        let mut handles = Vec::new();
        while let Some(read_trigger) = read_trigger_rx.recv().await {
            if self.can_decode(&self.buf[..]) {
                break;
            }
            if reader_index == self.readers.len() {
                break;
            }
            if !read_trigger {
                continue;
            }
            let idx = reader_index;
            reader_index += 1; // for next iter
            let shard_size = self.shard_size;
            let offset = self.offset;
            let read_trigger_tx = read_trigger_tx.clone();
            let (buf_idx, ref mut reader) = self.readers[idx];
            if reader.is_none() {
                read_trigger_tx.send(true).await;
                continue;
            }
            let reader = RawPtr(unsafe {
                reader.as_mut().unwrap() as *mut Box<dyn AsyncReadAt + Send + Unpin>
            });
            let buf = self.buf[buf_idx].get_or_insert_default();
            if buf.is_empty() {
                buf.resize(shard_size, 0u8);
            }
            let buf_ptr = RawPtr(unsafe { buf as *mut Vec<u8> });
            handles.push(tokio::spawn(async move {
                let buf = unsafe { buf_ptr.0.as_mut().unwrap() };
                let reader = unsafe { reader.0.as_mut().unwrap() };
                match reader.read_at(&mut buf[..], offset).await {
                    Err(e) => {
                        let mut err = None;
                        if let Some(eref) = e.get_ref() {
                            if let Some(eref) = eref.as_error::<StorageError>() {
                                if eref == &StorageError::FileNotFound
                                    || eref == &StorageError::FileCorrupt
                                {
                                    err = Some(e);
                                }
                            }
                        }

                        read_trigger_tx.send(true).await;
                        (Some(idx), err)
                    }
                    Ok(n) => {
                        buf.resize(n, 0u8); // shrink
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
                *self.readers[idx].1 = None;
            }
            if err.is_none() && r.1.is_some() {
                err = r.1;
            }
        }

        if self.can_decode(&self.buf[..]) {
            self.offset += self.shard_size as u64;
            if let Some(err) = err {
                return Err(err.into());
            }

            return Ok(&mut self.buf);
        }

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
    ) -> anyhow::Result<u64> {
        assert!(offset + length <= total_length);
        if length == 0 {
            return Ok(0);
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

            let buf = reader.read().await?;

            self.decode_data_blocks(buf)?;
        }

        Ok(0)
    }
}
