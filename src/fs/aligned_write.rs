use std::io::Error;
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;

use super::*;
use crate::prelude::Pin;

pub struct AlignedWriter<'a> {
    inner: File,
    aligned_buf: &'a mut [u8],
    total_size: u64,
}

impl<'a> AsyncWrite for AlignedWriter<'a> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        todo!()
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
