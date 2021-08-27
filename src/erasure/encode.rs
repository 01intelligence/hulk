use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use super::*;
use crate::errors;
use crate::io::AsyncReadFull;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erasure_encode() {
        const BLOCK_SIZE_V2: u64 = super::super::BLOCK_SIZE_V2 as u64;
        const ONE_MIB: u64 = 1 * crate::utils::MIB as u64;
        use crate::bitrot::DEFAULT_BITROT_ALGORITHM;

        struct Case {
            data_blocks: usize,
            on_disks: usize,
            off_disks: usize,
            block_size: u64,
            data: u64,
            offset: u64,
            algorithm: crate::bitrot::BitrotAlgorithm,
            should_fail: bool,
            should_fail_quorum: bool,
        }

        let cases = vec![
            Case {
                data_blocks: 2,
                on_disks: 4,
                off_disks: 0,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 0
            Case {
                data_blocks: 3,
                on_disks: 6,
                off_disks: 0,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 1,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 1
            Case {
                data_blocks: 4,
                on_disks: 8,
                off_disks: 2,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 2,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 2
            Case {
                data_blocks: 5,
                on_disks: 10,
                off_disks: 3,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: ONE_MIB,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 3
            Case {
                data_blocks: 6,
                on_disks: 12,
                off_disks: 4,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: ONE_MIB,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 4
            Case {
                data_blocks: 7,
                on_disks: 14,
                off_disks: 5,
                block_size: BLOCK_SIZE_V2,
                data: 0,
                offset: 0,
                should_fail: false,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail_quorum: false,
            }, // 5
            Case {
                data_blocks: 8,
                on_disks: 16,
                off_disks: 7,
                block_size: BLOCK_SIZE_V2,
                data: 0,
                offset: 0,
                should_fail: false,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail_quorum: false,
            }, // 6
            Case {
                data_blocks: 2,
                on_disks: 4,
                off_disks: 2,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: true,
            }, // 7
            Case {
                data_blocks: 4,
                on_disks: 8,
                off_disks: 4,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: true,
            }, // 8
            Case {
                data_blocks: 7,
                on_disks: 14,
                off_disks: 7,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: true,
            }, // 9
            Case {
                data_blocks: 8,
                on_disks: 16,
                off_disks: 8,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: true,
            }, // 10
            Case {
                data_blocks: 5,
                on_disks: 10,
                off_disks: 3,
                block_size: ONE_MIB,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 11
            Case {
                data_blocks: 3,
                on_disks: 6,
                off_disks: 1,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: ONE_MIB / 2,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 12
            Case {
                data_blocks: 2,
                on_disks: 4,
                off_disks: 0,
                block_size: ONE_MIB / 2,
                data: ONE_MIB,
                offset: ONE_MIB / 2 + 1,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 13
            Case {
                data_blocks: 4,
                on_disks: 8,
                off_disks: 0,
                block_size: ONE_MIB - 1,
                data: ONE_MIB,
                offset: ONE_MIB - 1,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 14
            Case {
                data_blocks: 8,
                on_disks: 12,
                off_disks: 2,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 2,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 15
            Case {
                data_blocks: 8,
                on_disks: 10,
                off_disks: 1,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 16
            Case {
                data_blocks: 10,
                on_disks: 14,
                off_disks: 0,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 17,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 17
            Case {
                data_blocks: 2,
                on_disks: 6,
                off_disks: 2,
                block_size: ONE_MIB,
                data: ONE_MIB,
                offset: ONE_MIB / 2,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: false,
            }, // 18
            Case {
                data_blocks: 10,
                on_disks: 16,
                off_disks: 8,
                block_size: BLOCK_SIZE_V2,
                data: ONE_MIB,
                offset: 0,
                algorithm: DEFAULT_BITROT_ALGORITHM,
                should_fail: false,
                should_fail_quorum: true,
            }, // 19
        ];
    }
}
