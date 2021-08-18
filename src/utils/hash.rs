use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

pub fn xx_hash(data: &[u8]) -> u64 {
    let mut hasher = BuildHasherDefault::<twox_hash::XxHash64>::default().build_hasher();
    hasher.write(data);
    hasher.finish()
}
