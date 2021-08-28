/// Buf guard which always return the same buf.
pub trait BufGuard {
    fn buf(&self) -> &[u8];
}

pub trait BufGuardMut: BufGuard {
    fn buf_mut(&mut self) -> &mut [u8];
}

/// Buf pool guard which always return a different buf.
pub trait BufPoolGuard {
    fn buf(&self) -> &[u8];
}

pub trait BufPoolGuardMut: BufPoolGuard {
    fn buf_mut(&mut self) -> &mut [u8];
}
