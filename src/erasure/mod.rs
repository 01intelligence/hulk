mod coding;
mod decode;
mod encode;
mod utils;

pub use coding::*;
pub use decode::*;
pub use encode::*;
pub use utils::*;

pub const BLOCK_SIZE_V1: usize = 10 * crate::utils::MIB;
pub const BLOCK_SIZE_V2: usize = 1 * crate::utils::MIB;

pub fn erasure_self_test() {}
