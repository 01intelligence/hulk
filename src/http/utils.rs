// The prefixes of used-defined metadata keys.
// All values stored with a key starting with one of the following prefixes
// must be extracted from the header.
pub const USER_METADATA_KEY_PREFIXES: [&str; 2] = ["x-amz-meta-", "x-hulk-meta-"];
