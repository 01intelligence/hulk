#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupType {
    Unknown,
    Fs,
    Erasure,
    DistributedErasure,
    Gateway,
}
