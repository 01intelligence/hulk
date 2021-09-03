use std::cell::{Ref, RefCell};
use std::sync::Arc;

use crate::object::ObjectInfo;
use crate::storage::{FileInfo, FileInfoVersions};
use crate::xl_storage::{VersionType, XlMetaV2};

pub struct MetaCacheEntry {
    pub name: String,
    // Use `Arc` to avoid copy overhead.
    pub metadata: Arc<Vec<u8>>,
    cached: RefCell<Option<FileInfo>>,
}

impl MetaCacheEntry {
    pub fn new(name: String, metadata: Arc<Vec<u8>>) -> Self {
        Self {
            name,
            metadata,
            cached: RefCell::new(None),
        }
    }

    fn is_dir(&self) -> bool {
        self.metadata.is_empty()
    }

    fn is_object(&self) -> bool {
        !self.metadata.is_empty()
    }

    fn has_prefix(&self, s: &str) -> bool {
        self.name.starts_with(s)
    }

    fn likely_matches(&self, other: &MetaCacheEntry) -> bool {
        // This should reject 99%.
        self.metadata.len() == other.metadata.len() && self.name == other.name
    }

    fn matches(&self, other: &MetaCacheEntry, bucket: &str) -> bool {
        if !self.likely_matches(other) {
            return false;
        }
        let a = self.file_info(bucket);
        let b = other.file_info(bucket);
        match a {
            Ok(a) => {
                let b = match b {
                    Ok(b) => b,
                    Err(_) => return false,
                };
                a.mod_time == b.mod_time && a.size == b.size && a.version_id == b.version_id
            }
            Err(_) => b.is_err(),
        }
    }

    fn is_in_dir(&self, dir: &str, separator: &str) -> bool {
        let ext: &str = if dir.is_empty() {
            // Root
            &self.name
        } else {
            let ext = self.name.trim_start_matches(dir);
            if ext.len() != self.name.len() {
                ext
            } else {
                return false;
            }
        };

        // Separator is not found or is the last entry.
        match ext.find(separator) {
            None => true,
            Some(idx) => idx + separator.as_bytes().len() == ext.as_bytes().len(),
        }
    }

    fn is_latest_delete_marker(&self) -> bool {
        let fi_ref = self.cached.borrow();
        if let Some(fi) = fi_ref.as_ref() {
            return fi.deleted;
        }
        match XlMetaV2::load_with_data(&self.metadata) {
            Err(_) => true, // TODO
            Ok(xl_meta) => {
                xl_meta.versions.is_empty()
                    || xl_meta.versions.last().unwrap().type_ == VersionType::Delete
            }
        }
    }

    fn file_info(&self, bucket: &str) -> anyhow::Result<Ref<FileInfo>> {
        let mut fi_ref = self.cached.borrow_mut();
        if fi_ref.is_none() {
            let fi = if self.is_dir() {
                FileInfo {
                    volume: bucket.to_owned(),
                    name: self.name.to_owned(),
                    ..Default::default()
                }
            } else {
                crate::xl_storage::get_file_info(&self.metadata, bucket, &self.name, "", false)?
            };
            *fi_ref = Some(fi);
        }
        Ok(Ref::map(self.cached.borrow(), |fi| {
            let fi = fi.as_ref().unwrap();
            assert_eq!(&fi.volume, bucket);
            fi
        }))
    }

    fn file_info_versions(&self, bucket: &str) -> anyhow::Result<FileInfoVersions> {
        if self.is_dir() {
            Ok(FileInfoVersions {
                volume: bucket.to_owned(),
                name: self.name.clone(),
                is_empty_dir: false,
                latest_mod_time: Default::default(),
                versions: vec![FileInfo {
                    volume: bucket.to_owned(),
                    name: self.name.clone(),
                    ..Default::default()
                }],
            })
        } else {
            crate::xl_storage::get_file_info_versions(&self.metadata, bucket, &self.name)
        }
    }
}

pub struct MetaCacheEntries(Vec<MetaCacheEntry>);

impl MetaCacheEntries {
    fn sort(&mut self) {
        if !self.is_sorted() {
            self.0.sort_unstable_by(|a, b| a.name.cmp(&b.name))
        }
    }

    fn is_sorted(&self) -> bool {
        self.0.is_sorted_by(|a, b| a.name.partial_cmp(&b.name))
    }

    fn resolve(&self, params: &MetadataResolutionParams) -> Option<MetaCacheEntry> {
        todo!()
    }

    fn first_found(&self) -> (MetaCacheEntry, usize) {
        todo!()
    }

    fn names(&self) -> Vec<String> {
        todo!()
    }
}

pub struct MetadataResolutionParams {
    pub dir_quorum: usize,
    pub obj_quorum: usize,
    pub bucket: String,
}

pub struct MetaCacheEntriesSorted {
    entries: MetaCacheEntries,
    list_id: String,
}

impl MetaCacheEntriesSorted {
    fn file_info_versions(
        &self,
        bucket: &str,
        prefix: &str,
        delimiter: &str,
        after_v: &str,
    ) -> Vec<ObjectInfo> {
        todo!()
    }

    fn file_infos(&self, bucket: &str, prefix: &str, delimiter: &str) -> Vec<ObjectInfo> {
        todo!()
    }

    fn forward_to(&mut self, s: &str) {
        todo!()
    }

    fn forward_past(&mut self, s: &str) {
        todo!()
    }

    fn merge(&mut self, other: &MetaCacheEntriesSorted, limit: usize) {
        todo!()
    }

    fn filter_prefix(&mut self, s: &str) {
        todo!()
    }

    fn filter_objects_only(&mut self, s: &str) {
        todo!()
    }

    fn filter_prefixes_only(&mut self, s: &str) {
        todo!()
    }

    fn filter_recursive_entries(&mut self, prefix: &str, separator: &str) {
        todo!()
    }

    fn truncate(&mut self, n: usize) {
        todo!()
    }

    fn len(&self) -> usize {
        self.entries.0.len()
    }

    fn entries(&self) -> &MetaCacheEntries {
        &self.entries
    }

    fn deduplicate<F>(compare: F) -> bool
    where
        F: Fn(&MetaCacheEntry, &MetaCacheEntry) -> bool,
    {
        todo!()
    }
}
