use std::future::Future;
use std::io::ErrorKind;
use std::sync::Arc;

use bytes::BufMut;
use futures_util::ready;
use tokio::io::AsyncWrite;

use super::*;
use crate::prelude::*;
use crate::utils;
use crate::utils::BufGuardMut;

#[pin_project::pin_project(project = AlignedWriterProj)]
pub struct AlignedWriter<G: BufGuardMut> {
    std: Arc<std::fs::File>,
    total_size: Option<u64>,
    buffer: &'static mut [u8],
    buffer_cursor: &'static mut [u8],
    #[pin]
    buf_guard: G,
    written: u64,
    state: State,
}

enum State {
    Idle,
    Buffering,
    Busy(usize, tokio::task::JoinHandle<std::io::Result<()>>),
}

impl<G: BufGuardMut> AlignedWriter<G> {
    /// Write using aligned buffer.
    ///
    /// If [`total_size`] is not [`None`], control total size to write.
    /// Note that [`aligned_buffer`] must be aligned to [`DIRECTIO_ALIGN_SIZE`] page boundaries.
    /// File [`f`] must be opened with DIRECT I/O enabled.
    /// Caller must call [`tokio::io::AsyncWriteExt::flush`] after all writes.
    pub fn new(f: std::fs::File, mut aligned_buf_guard: G, total_size: Option<u64>) -> Self {
        let buffer = aligned_buf_guard.buf_mut();
        // Safety: lifetime of `buf` depends on `buf_guard`.
        let buffer = unsafe { std::slice::from_raw_parts_mut(buffer.as_mut_ptr(), buffer.len()) };
        assert_eq!(buffer.len() % DIRECTIO_ALIGN_SIZE, 0);
        AlignedWriter {
            std: Arc::new(f),
            total_size,
            buffer,
            buffer_cursor: &mut [],
            buf_guard: aligned_buf_guard,
            written: 0,
            state: State::Idle,
        }
    }

    pub fn into_std(self) -> std::fs::File {
        Arc::try_unwrap(self.std).expect("Arc::try_unwrap failed")
    }

    pub async fn sync_data(&self) -> std::io::Result<()> {
        let std = self.std.clone();
        asyncify(move || std.sync_data()).await
    }

    pub async fn sync_all(&self) -> std::io::Result<()> {
        let std = self.std.clone();
        asyncify(move || std.sync_all()).await
    }

    fn write(
        this: &AlignedWriterProj<'_, G>,
        size: usize,
    ) -> std::io::Result<tokio::task::JoinHandle<std::io::Result<()>>> {
        if this.buffer.len() % DIRECTIO_ALIGN_SIZE != 0 {
            this.std.disable_direct_io()?; // TODO: async
        }

        // TODO: should we use std file blocking read or tokio-uring?
        let mut std = this.std.clone();
        let buf_ptr = utils::SendRawPtr::new(this.buffer.as_ptr());
        let rx = tokio::task::spawn_blocking(move || {
            // Safety: buffer may be invalidated somewhere,
            // which may lead to dirty stuff to be written to file,
            // but even without this, the file is broken anyway, since writing has been cancelled.
            let mut buffer = unsafe { std::slice::from_raw_parts(buf_ptr.to(), size) };
            (&mut &*std).write_all(buffer)
        });
        Ok(rx)
    }
}

impl<G: BufGuardMut> AsyncWrite for AlignedWriter<G> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.project();
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if let Some(total_size) = this.total_size {
            if this.written == total_size {
                return Poll::Ready(Ok(0)); // deny writing anymore
            }
        }

        loop {
            match this.state {
                State::Idle => {
                    // Assign buffer.
                    if let Some(total_size) = this.total_size {
                        let remaining = (*total_size - *this.written) as usize;
                        if remaining < this.buffer.len() {
                            let buffer = &mut this.buffer[..remaining];
                            // Safety: bypass lifetime check.
                            *this.buffer = unsafe {
                                std::slice::from_raw_parts_mut(buffer.as_mut_ptr(), buffer.len())
                            };
                        }
                    }
                    // Safety: bypass lifetime check.
                    *this.buffer_cursor = unsafe {
                        std::slice::from_raw_parts_mut(this.buffer.as_mut_ptr(), this.buffer.len())
                    };

                    *this.state = State::Buffering;
                }
                State::Buffering => {
                    let consume = buf.len().min(this.buffer_cursor.remaining_mut());
                    debug_assert_ne!(consume, 0);
                    this.buffer_cursor.put_slice(&buf[..consume]);
                    *this.written += consume as u64;
                    if this.buffer_cursor.has_remaining_mut() {
                        return Poll::Ready(Ok(consume));
                    }

                    // Buffer is full, so write it.
                    let rx = Self::write(&this, this.buffer.len())?;

                    *this.state = State::Busy(consume, rx);
                }
                State::Busy(consume, ref mut rx) => {
                    let consume = *consume;
                    let res = ready!(Pin::new(rx).poll(cx))?;
                    res?;
                    *this.state = State::Idle;
                    return Poll::Ready(Ok(consume));
                }
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.project();
        loop {
            match this.state {
                State::Idle => break,
                State::Buffering => {
                    let buf_size = this.buffer.len() - this.buffer_cursor.remaining_mut();
                    if buf_size > 0 {
                        let rx = Self::write(&this, buf_size)?;

                        *this.state = State::Busy(0, rx);
                    } else {
                        break;
                    }
                }
                State::Busy(_, ref mut rx) => {
                    let res = ready!(Pin::new(rx).poll(cx))?;
                    res?;
                    *this.state = State::Idle;
                    break;
                }
            };
        }
        return Poll::Ready(Ok(()));
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use scopeguard::defer_on_success;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::utils;

    #[test]
    fn test_aligned_writer() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap();

        runtime.block_on(async {
            use utils::Rng;
            let rnd = utils::rng_seed_now().gen::<[u8; 8]>();
            let tmp_file = format!(".test_aligned_writer-{}.tmp", hex::encode(rnd));

            aligned_write_and_read_numbers(
                &tmp_file,
                utils::MIB,
                DIRECTIO_ALIGN_SIZE,
                4 * DIRECTIO_ALIGN_SIZE,
                None,
            )
            .await;
            aligned_write_and_read_numbers(
                &tmp_file,
                utils::MIB,
                4 * DIRECTIO_ALIGN_SIZE,
                DIRECTIO_ALIGN_SIZE,
                None,
            )
            .await;
            aligned_write_and_read_numbers(
                &tmp_file,
                utils::MIB + utils::KIB,
                4 * DIRECTIO_ALIGN_SIZE,
                DIRECTIO_ALIGN_SIZE,
                None,
            )
            .await;
            aligned_write_and_read_numbers(
                &tmp_file,
                utils::MIB + utils::KIB,
                DIRECTIO_ALIGN_SIZE - 1,
                4 * DIRECTIO_ALIGN_SIZE,
                None,
            )
            .await;
        });
    }

    async fn aligned_write_and_read_numbers(
        tmp_file: &str,
        numbers: usize,
        chunk_size: usize,
        aligned_buf_size: usize,
        total_size: Option<usize>,
    ) {
        let aligned_buf = AlignedBlock::new(aligned_buf_size);

        impl utils::BufGuard for AlignedBlock {
            fn buf(&self) -> &[u8] {
                self
            }
        }

        impl utils::BufGuardMut for AlignedBlock {
            fn buf_mut(&mut self) -> &mut [u8] {
                self
            }
        }

        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open_direct_io(&tmp_file)
            .await
            .unwrap();

        defer_on_success! {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    remove(tmp_file).await.unwrap();
                });
            });
        }

        let mut writer = AlignedWriter::new(file.into_std().await, aligned_buf, None);

        let content = (0..numbers)
            .map(|n| format!("{:08}", n).into_bytes())
            .flatten();
        for chunk in &content.chunks(chunk_size) {
            let buf: Vec<u8> = chunk.collect();
            writer.write_all(&buf[..]).await.unwrap();
        }
        writer.flush().await.unwrap();

        let meta = tokio::fs::metadata(&tmp_file).await.unwrap();
        assert_eq!(meta.len(), numbers as u64 * 8);

        let content = tokio::fs::read(&tmp_file).await.unwrap();
        assert_eq!(content.len(), numbers * 8);
        for (i, chunk) in content.chunks(8).enumerate() {
            let got = String::from_utf8_lossy(chunk).parse::<usize>().unwrap();
            assert_eq!(got, i);
        }
    }
}
