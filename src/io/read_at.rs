use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt};

use super::AsyncReadFull;

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
