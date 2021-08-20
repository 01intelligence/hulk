use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

pub fn xx_hash(data: &[u8]) -> u64 {
    let mut hasher = BuildHasherDefault::<twox_hash::XxHash64>::default().build_hasher();
    hasher.write(data);
    hasher.finish()
}

pub struct XxHashReader<'a, R: std::io::Read> {
    inner: &'a mut R,
    hasher: twox_hash::XxHash64,
}

impl<'a, R: std::io::Read> XxHashReader<'a, R> {
    pub fn new(r: &'a mut R) -> Self {
        Self {
            inner: r,
            hasher: BuildHasherDefault::<twox_hash::XxHash64>::default().build_hasher(),
        }
    }

    pub fn hash(&self) -> u64 {
        self.hasher.finish()
    }
}

impl<'a, R: std::io::Read> std::io::Read for XxHashReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.hasher.write(&buf[..n]);
        }
        Ok(n)
    }
}
