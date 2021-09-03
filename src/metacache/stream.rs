use std::sync::Arc;

use snap::read::FrameDecoder;
use snap::write::FrameEncoder;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

use super::*;

pub const METACACHE_STREAM_VERSION: u8 = 1;

pub struct MetaCacheWriter<W: std::io::Write + Send + 'static> {
    inner: Option<W>,
    writer: Option<FrameEncoder<W>>,
}

impl<W: std::io::Write + Send + 'static> MetaCacheWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            inner: Some(writer),
            writer: None,
        }
    }

    fn prepare(&mut self) -> anyhow::Result<()> {
        if self.writer.is_none() {
            // TODO: reuse `FrameEncoder`
            let mut w = FrameEncoder::new(self.inner.take().unwrap());
            rmp::encode::write_u8(&mut w, METACACHE_STREAM_VERSION)?;
            self.writer = Some(w);
        }
        Ok(())
    }

    pub fn reset(&mut self, writer: W) {
        self.inner = Some(writer);
        self.writer = None;
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        if let Some(w) = &mut self.writer {
            rmp::encode::write_bool(w, false)?;
            use std::io::Write;
            w.flush()?;
        }
        Ok(())
    }

    pub fn write(&mut self, entries: &[&MetaCacheEntry]) -> anyhow::Result<()> {
        self.prepare()?;

        let w = self.writer.as_mut().unwrap();
        for entry in entries {
            assert!(!entry.name.is_empty());
            rmp::encode::write_bool(w, true)?;
            rmp::encode::write_str(w, &entry.name)?;
            rmp::encode::write_bin(w, &entry.metadata)?;
        }
        Ok(())
    }

    pub fn write_sender(
        mut self,
    ) -> anyhow::Result<(JoinHandle<anyhow::Result<Self>>, Sender<MetaCacheEntry>)> {
        self.prepare()?;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<MetaCacheEntry>(100);
        let handle = tokio::spawn(async move {
            let w = self.writer.as_mut().unwrap();
            while let Some(entry) = rx.recv().await {
                assert!(!entry.name.is_empty());
                rmp::encode::write_bool(w, true)?;
                rmp::encode::write_str(w, &entry.name)?;
                rmp::encode::write_bin(w, &entry.metadata)?;
            }
            Ok(self)
        });
        Ok((handle, tx))
    }
}

pub struct MetaCacheReader<R: std::io::Read + Send + 'static> {
    inner: Option<R>,
    reader: Option<FrameDecoder<R>>,
}

impl<R: std::io::Read + Send + 'static> MetaCacheReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            inner: Some(reader),
            reader: None,
        }
    }

    fn prepare(&mut self) -> anyhow::Result<()> {
        if self.reader.is_none() {
            // TODO: reuse `FrameDecoder`
            let mut r = FrameDecoder::new(self.inner.take().unwrap());
            let v = rmp::decode::read_u8(&mut r)?;
            if v == METACACHE_STREAM_VERSION {
                anyhow::bail!("MetaCacheReader unknown version '{}'", v);
            }
            self.reader = Some(r);
        }
        Ok(())
    }

    pub fn read(&mut self) -> anyhow::Result<Option<MetaCacheEntry>> {
        self.prepare()?;

        let r = self.reader.as_mut().unwrap();

        let more = rmp::decode::read_bool(r)?;
        if !more {
            return Ok(None);
        }

        let name_len = rmp::decode::read_str_len(r)?;
        let mut name = vec![0u8; name_len as usize];
        let _ = rmp::decode::read_str_data(r, name_len, &mut name).map_err(|err| {
            // Bypass 'static lifetime issue of `err`.
            anyhow::anyhow!("read_str_data '{}'", err)
        })?;
        // Safety: it has been checked above.
        let name = unsafe { String::from_utf8_unchecked(name) };

        let mut metadata_len = rmp::decode::read_bin_len(r)?;
        use std::io::Read;
        let mut metadata = vec![0u8; metadata_len as usize];
        r.read_exact(&mut metadata)?;

        let entry = MetaCacheEntry::new(name, Arc::new(metadata));

        Ok(Some(entry))
    }

    pub fn read_receiver(
        mut self,
    ) -> anyhow::Result<(JoinHandle<anyhow::Result<Self>>, Receiver<MetaCacheEntry>)> {
        let (tx, rx) = tokio::sync::mpsc::channel::<MetaCacheEntry>(100);
        let handle = tokio::spawn(async move {
            loop {
                if let Some(entry) = self.read()? {
                    if let Err(_) = tx.send(entry).await {
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok(self)
        });
        Ok((handle, rx))
    }

    pub fn read_eof(&mut self) -> bool {
        if let Ok(None) = self.read() {
            true
        } else {
            false
        }
    }

    pub fn skip(&mut self, mut n: usize) -> anyhow::Result<Option<()>> {
        while n > 0 {
            n -= 1;

            if let None = self.read()? {
                return Ok(None);
            }
        }

        Ok(Some(()))
    }

    pub fn forward_to(&mut self, s: &str) -> anyhow::Result<()> {
        todo!()
    }

    pub fn read_n(&mut self, n: usize, include_deleted: bool, include_dirs: bool, prefix: &str) {
        todo!()
    }
}
