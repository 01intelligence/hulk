use std::fmt;
use std::hash::{Hash, Hasher};

use anyhow::private::kind::AdhocKind;
use thiserror::Error;

use super::StorageError;
use crate::prelude::HashMap;

#[derive(Debug, Error)]
pub struct ReducibleError {
    ident: u8,
    inner: ReducibleErrorInner,
}

#[derive(Debug, Error)]
#[non_exhaustive]
enum ReducibleErrorInner {
    IoError(std::io::Error),
    StorageError(StorageError),
}

impl From<std::io::Error> for ReducibleError {
    fn from(err: std::io::Error) -> Self {
        Self {
            ident: 0,
            inner: ReducibleErrorInner::IoError(err),
        }
    }
}

impl From<StorageError> for ReducibleError {
    fn from(err: StorageError) -> Self {
        Self {
            ident: 1,
            inner: ReducibleErrorInner::StorageError(err),
        }
    }
}

impl fmt::Display for ReducibleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl fmt::Display for ReducibleErrorInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ReducibleErrorInner::*;
        match &self {
            IoError(err) => err.fmt(f),
            StorageError(err) => err.fmt(f),
        }
    }
}

impl PartialEq for ReducibleError {
    fn eq(&self, other: &Self) -> bool {
        use ReducibleErrorInner::*;
        if self.ident != other.ident {
            return false;
        }
        match &self.inner {
            IoError(err) => {
                if let IoError(e) = &other.inner {
                    return err.kind() == e.kind();
                }
            }
            StorageError(err) => {
                if let StorageError(e) = &other.inner {
                    return err == e;
                }
            }
        }
        false
    }
}

impl Eq for ReducibleError {}

impl Hash for ReducibleError {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use ReducibleErrorInner::*;
        self.ident.hash(state);
        match &self.inner {
            IoError(err) => err.kind().hash(state),
            StorageError(err) => err.hash(state),
        }
    }
}

impl ReducibleError {
    pub fn is(&self, errs: &[ReducibleError]) -> bool {
        use ReducibleErrorInner::*;
        return errs.iter().any(|e| {
            match &e.inner {
                IoError(e) => {
                    if let IoError(err) = &self.inner {
                        return err.kind() == e.kind();
                    }
                }
                StorageError(e) => {
                    if let StorageError(err) = &self.inner {
                        return err == e;
                    }
                }
            }
            false
        });
    }
}

pub fn count_none(errs: &[Option<ReducibleError>]) -> usize {
    errs.iter().filter(|e| e.is_none()).count()
}

pub fn count_err(errs: &[Option<ReducibleError>], err: &ReducibleError) -> usize {
    errs.iter().fold(0, |acc, e| {
        if let Some(e) = e {
            if e == err {
                return acc + 1;
            }
        }
        acc
    })
}

pub fn reduce_errs<'a>(
    errs: Vec<Option<ReducibleError>>,
    ignored_errs: &[ReducibleError],
) -> (usize, Option<ReducibleError>) {
    let mut err_counts = HashMap::new();
    for err in errs {
        if let Some(err) = &err {
            if err.is(ignored_errs) {
                continue;
            }
        }
        *err_counts.entry(err).or_default() += 1;
    }

    let mut max = 0usize;
    let mut max_err = None;
    for (err, count) in err_counts {
        if max < count {
            max = count;
            max_err = err;
        } else if max == count && err.is_none() {
            // Prefer `None` over other error values with the same
            // number of occurrences.
            max_err = None;
        }
    }
    (max, max_err)
}
