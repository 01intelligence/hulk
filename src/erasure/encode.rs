use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use super::*;
use crate::errors;
use crate::utils::AsyncReadExt;

struct ParallelWriter<'a> {
    writers: &'a [&'a mut Box<dyn AsyncWrite + Unpin>],
    write_quorum: usize,
    errors: Vec<Option<errors::ReducibleError>>,
}

impl<'a> ParallelWriter<'a> {
    async fn write(&mut self, blocks: &[&mut [u8]]) -> anyhow::Result<()> {
        let mut tasks = Vec::new();
        for i in 0..self.writers.len() {
            // Safety: borrows splitting.
            let mut writer = unsafe {
                (self.writers.as_ptr().add(i) as *mut &'a mut Box<dyn AsyncWrite + Unpin>)
                    .as_mut()
                    .unwrap()
            };
            let error = unsafe { self.errors.as_mut_ptr().add(i).as_mut().unwrap() };
            if error.is_some() {
                continue;
            }
            tasks.push(async move {
                match writer.write_all(blocks[i]).await {
                    Err(err) => {
                        *error = Some(err.into());
                    }
                    Ok(_) => {}
                }
            })
        }
        let _ = futures_util::future::join_all(tasks).await;
        if crate::errors::count_none(&self.errors[..]) >= self.write_quorum {
            return Ok(());
        }
        let errors = std::mem::take(&mut self.errors);
        reduce_write_quorum_errs(
            errors,
            &super::OBJECT_OP_IGNORED_ERRORS[..],
            self.write_quorum,
        )
        .map_or_else(|| Ok(()), |e| Err(e.into()))
    }
}

impl Erasure {
    pub async fn encode(
        &self,
        src: &mut (impl AsyncRead + Unpin),
        writers: &[&mut Box<dyn AsyncWrite + Unpin>],
        buf: &mut Vec<u8>,
        quorum: usize,
    ) -> anyhow::Result<u64> {
        let mut writer = ParallelWriter {
            writers,
            write_quorum: quorum,
            errors: (0..writers.len()).map(|_| None).collect(),
        };

        let mut total = 0;
        loop {
            let n = src.read_full(&mut buf[..]).await?;
            let eof = n < buf.len();
            if n == 0 && total != 0 {
                break;
            }
            let blocks = self.encode_data(buf)?;
            writer.write(&blocks[..]).await?;
            total += n as u64;
            if eof {
                break;
            }
        }

        Ok(total)
    }
}
