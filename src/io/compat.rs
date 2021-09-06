use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct AsyncWriteStdWriter<W: AsyncWrite + Unpin> {
    w: W,
}

impl<W: AsyncWrite + Unpin> AsyncWriteStdWriter<W> {
    pub fn new(w: W) -> Self {
        Self { w }
    }
}

impl<W: AsyncWrite + Unpin> std::io::Write for AsyncWriteStdWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        tokio::runtime::Handle::current().block_on(async { self.w.write(buf).await })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        tokio::runtime::Handle::current().block_on(async { self.w.flush().await })
    }
}

pub struct AsyncReadStdReader<R: AsyncRead + Unpin> {
    r: R,
}

impl<R: AsyncRead + Unpin> AsyncReadStdReader<R> {
    pub fn new(r: R) -> Self {
        Self { r }
    }
}

impl<R: AsyncRead + Unpin> std::io::Read for AsyncReadStdReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        tokio::runtime::Handle::current().block_on(async { self.r.read(buf).await })
    }
}
