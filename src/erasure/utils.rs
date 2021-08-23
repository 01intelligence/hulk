use lazy_static::lazy_static;

use crate::errors::{self, ReducibleError, StorageError};

lazy_static! {
    pub static ref OBJECT_OP_IGNORED_ERRORS: Vec<ReducibleError> = {
        let mut errs: Vec<ReducibleError> = errors::BASE_STORAGE_ERRORS
            .iter()
            .cloned()
            .map(|e| e.into())
            .collect();
        errs.push(StorageError::DiskAccessDenied.into());
        errs.push(StorageError::UnformattedDisk.into());
        errs
    };
}

pub(super) fn reduce_quorum_errs(
    errs: Vec<Option<ReducibleError>>,
    ignored_errs: &[ReducibleError],
    quorum: usize,
) -> Option<ReducibleError> {
    let (max_count, max_err) = crate::errors::reduce_errs(errs, ignored_errs);
    if max_count >= quorum {
        return max_err;
    }
    None
}

pub(super) fn reduce_read_quorum_errs(
    errs: Vec<Option<ReducibleError>>,
    ignored_errs: &[ReducibleError],
    read_quorum: usize,
) -> Option<ReducibleError> {
    reduce_quorum_errs(errs, ignored_errs, read_quorum)
}

pub(super) fn reduce_write_quorum_errs(
    errs: Vec<Option<ReducibleError>>,
    ignored_errs: &[ReducibleError],
    write_quorum: usize,
) -> Option<ReducibleError> {
    reduce_quorum_errs(errs, ignored_errs, write_quorum)
}
