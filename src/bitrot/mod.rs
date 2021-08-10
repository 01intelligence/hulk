mod highway;

pub use self::highway::*;

pub fn bitrot_self_test() {}

pub enum BitrotAlgorithm {
    HighwayHash256,
}

pub struct BitrotVerifier {}
