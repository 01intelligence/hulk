pub struct WalkDirOptions {
    /// Bucket.
    pub bucket: String,
    /// Director in the bucket.
    pub base_dir: String,
    /// Full recursive walk.
    pub recursive: bool,
    /// Return not-found error if all disks reports that `base_dir` cannot be found.
    pub report_not_found: bool,
    /// Only return results with given prefix within directory.
    /// Should never contain a slash.
    pub filter_prefix: String,
    /// Forward to the given object path.
    pub forward_to: String,
}
