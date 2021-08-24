mod highway;

pub use self::highway::*;

pub fn bitrot_self_test() {}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum BitrotAlgorithm {
    HighwayHash256,
}

pub struct BitrotVerifier {}

pub const DEFAULT_BITROT_ALGORITHM: BitrotAlgorithm = BitrotAlgorithm::HighwayHash256;
