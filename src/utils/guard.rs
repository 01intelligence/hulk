/// Buf guard which always return the same buf.
pub trait BufGuard {
    fn buf(&self) -> &[u8];
}

pub trait BufGuardMut: BufGuard {
    fn buf_mut(&mut self) -> &mut [u8];
}

pub enum EitherGuard<T1, T2> {
    Left(T1),
    Right(T2),
}

impl<T1: BufGuard, T2: BufGuard> BufGuard for EitherGuard<T1, T2> {
    fn buf(&self) -> &[u8] {
        match self {
            EitherGuard::Left(g) => g.buf(),
            EitherGuard::Right(g) => g.buf(),
        }
    }
}

impl<T1: BufGuardMut, T2: BufGuardMut> BufGuardMut for EitherGuard<T1, T2> {
    fn buf_mut(&mut self) -> &mut [u8] {
        match self {
            EitherGuard::Left(g) => g.buf_mut(),
            EitherGuard::Right(g) => g.buf_mut(),
        }
    }
}
