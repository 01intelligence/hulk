/// Raw pointer workaround for "cannot be sent between threads safely".
/// Use with caution!
pub struct SendRawPtr<P>(P);

impl<P> SendRawPtr<P> {
    pub fn new(p: P) -> Self {
        Self(p)
    }
    pub fn to(self) -> P {
        self.0
    }
}

unsafe impl<T> Send for SendRawPtr<T> {}
