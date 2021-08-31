/// Raw pointer workaround for "cannot be sent between threads safely".
/// Use with caution!
#[derive(Copy, Clone)]
pub struct SendRawPtr<P: Copy>(P);

impl<P: Copy> SendRawPtr<P> {
    pub fn new(p: P) -> Self {
        Self(p)
    }
    pub fn to(self) -> P {
        self.0
    }
}

unsafe impl<P: Copy> Send for SendRawPtr<P> {}
unsafe impl<P: Copy> Sync for SendRawPtr<P> {}
