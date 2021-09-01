use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::Display;

use super::{is_null_version_id, StorageError};
use crate::fs::StdOpenOptionsNoAtime;
use crate::prelude::*;
use crate::utils;
use crate::utils::{DateTimeExt, StrExt};

const XL_HEADER: &[u8; 4] = b"XL2 ";

// Breaking changes.
// Newer versions cannot be read by older software.
// This will prevent downgrades to incompatible versions.
const XL_VERSION_MAJOR: u16 = 1;

// Non breaking changes.
// Bumping this is informational, but should be done
// if any change is made to the data stored, bumping this
// will allow to detect the exact version later.
const XL_VERSION_MINOR: u16 = 2;

lazy_static! {
    static ref XL_VERSION_CURRENT: [u8; 4] = {
        let mut version = [0u8; 4];
        (&mut version[..2])
            .write_u16::<LittleEndian>(XL_VERSION_MAJOR)
            .unwrap();
        (&mut version[2..])
            .write_u16::<LittleEndian>(XL_VERSION_MINOR)
            .unwrap();
        version
    };
}

fn check_xl2_v1(buf: &[u8]) -> anyhow::Result<(&[u8], u16, u16)> {
    if buf.len() <= 8 {
        anyhow::bail!("xl.meta: no data");
    }
    if &buf[..4] != &XL_HEADER[..] {
        anyhow::bail!(
            "xl.meta: unknown XLv2 header, expected {:?}, got {:?}",
            XL_HEADER,
            &buf[..4]
        );
    }
    let (major, minor) = if &buf[4..8] == b"1   " {
        (1, 0)
    } else {
        (
            (&mut &buf[4..6]).read_u16::<LittleEndian>().unwrap(),
            (&mut &buf[6..8]).read_u16::<LittleEndian>().unwrap(),
        )
    };
    if major > XL_VERSION_MAJOR {
        anyhow::bail!("xl.meta: unknown major version {} found", major);
    }
    Ok((&buf[8..], major, minor))
}

fn is_xl2_v1_format(buf: &[u8]) -> bool {
    check_xl2_v1(buf).is_err()
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum VersionType {
    Object = 1,
    Delete = 2,
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, Debug, PartialEq, Display)]
#[repr(u8)]
pub enum ErasureAlgo {
    #[strum(serialize = "reedsolomn")]
    ReedSolomon = 1,
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum ChecksumAlgo {
    HighwayHash = 1,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct XlMetaV2DeleteMarker {
    #[serde(rename = "ID")]
    version_id: Option<uuid::Uuid>,
    #[serde(rename = "MTime")]
    mod_time: i64,
    #[serde(rename = "MetaSys")]
    meta_sys: HashMap<String, String>, // internal metadata
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct XlMetaV2Object {
    #[serde(rename = "ID")]
    version_id: Option<uuid::Uuid>,
    #[serde(rename = "DDir")]
    data_dir: Option<uuid::Uuid>,
    #[serde(rename = "EcAlgo")]
    erasure_algorithm: ErasureAlgo,
    #[serde(rename = "EcM")]
    erasure_m: usize,
    #[serde(rename = "EcN")]
    erasure_n: usize,
    #[serde(rename = "EcBSize")]
    erasure_block_size: u64,
    #[serde(rename = "EcIndex")]
    erasure_index: usize,
    #[serde(rename = "EcDist")]
    erasure_distribution: Vec<u8>,
    #[serde(rename = "CSumAlgo")]
    bitrot_checksum_algo: ChecksumAlgo,
    #[serde(rename = "PartNums")]
    part_numbers: Vec<usize>,
    #[serde(rename = "PartETags")]
    part_etags: Vec<String>,
    #[serde(rename = "PartSizes")]
    part_sizes: Vec<u64>,
    #[serde(rename = "PartASizes")]
    part_actual_sizes: Vec<i64>,
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "MTime")]
    mod_time: i64,
    #[serde(rename = "MetaSys")]
    meta_sys: HashMap<String, String>,
    // internal metadata
    #[serde(rename = "MetaUsr")]
    meta_user: HashMap<String, String>, // metadata set by user
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct XlMetaV2Version {
    #[serde(rename = "Type")]
    type_: VersionType,
    #[serde(rename = "V2Obj")]
    object_v2: Option<XlMetaV2Object>,
    #[serde(rename = "DelObj")]
    delete_marker: Option<XlMetaV2DeleteMarker>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct XlMetaV2 {
    #[serde(rename = "Versions")]
    versions: Vec<XlMetaV2Version>,
    #[serde(skip)]
    pub data: HashMap<String, Vec<u8>>,
}

impl XlMetaV2Version {
    fn valid(&self) -> bool {
        match self.type_ {
            VersionType::Object => {
                if let Some(object_v2) = &self.object_v2 {
                    (object_v2.erasure_m >= object_v2.erasure_n)
                        && (object_v2.erasure_m != 0)
                        && (object_v2.erasure_n != 0)
                } else {
                    false
                }
            }
            VersionType::Delete => {
                if let Some(delete_marker) = &self.delete_marker {
                    delete_marker.mod_time > 0
                } else {
                    false
                }
            }
        }
    }
}

impl XlMetaV2Object {
    fn uses_data_dir(&self) -> bool {
        let key = String::from(crate::globals::RESERVED_METADATA_PREFIX_LOWER)
            + crate::bucket::TRANSITION_STATUS;
        if &crate::bucket::TransitionStatus::Complete.to_string()
            != self
                .meta_sys
                .get(&key)
                .map(|v| v as &str)
                .unwrap_or_else(|| "")
        {
            return true;
        }
        crate::bucket::is_restored_object_on_disk(&self.meta_user)
    }

    fn to_file_info(&self, volume: &str, path: &str) -> anyhow::Result<crate::storage::FileInfo> {
        let mut parts = Vec::with_capacity(self.part_numbers.len());
        let mut checksums = Vec::with_capacity(parts.capacity());
        for i in 0..self.part_numbers.len() {
            parts.push(super::ObjectPartInfo {
                etag: self.part_etags[i].clone(),
                number: self.part_numbers[i],
                size: self.part_sizes[i],
                actual_size: self.part_actual_sizes[i],
            });
            let mut checksum = match self.bitrot_checksum_algo {
                ChecksumAlgo::HighwayHash => super::ChecksumInfo {
                    part_number: parts[i].number,
                    algorithm: crate::bitrot::BitrotAlgorithm::HighwayHash256,
                    hash: vec![],
                },
            };
            checksums.push(checksum);
        }
        let erasure = super::ErasureInfo {
            algorithm: self.erasure_algorithm.to_string(),
            data_blocks: self.erasure_m,
            parity_blocks: self.erasure_n,
            block_size: self.erasure_block_size,
            index: self.erasure_index,
            distribution: self.erasure_distribution.clone(),
            checksums,
        };
        let mut metadata = HashMap::with_capacity(self.meta_sys.len() + self.meta_user.len());
        let mut version_purge_status = None;
        for (k, v) in &self.meta_sys {
            if k.in_ignore_ascii_case(&[crate::storage::VERSION_PURGE_STATUS_KEY]) {
                version_purge_status = Some(crate::storage::VersionPurgeStatus::from_str(v)?);
            } else if k
                .to_lowercase()
                .starts_with(crate::globals::RESERVED_METADATA_PREFIX_LOWER)
            {
                metadata.insert(k.clone(), v.clone());
            }
        }
        for (k, v) in &self.meta_user {
            if k.in_ignore_ascii_case(&[
                crate::http::AMZ_META_UNENCRYPTED_CONTENT_LENGTH,
                crate::http::AMZ_META_UNENCRYPTED_CONTENT_MD5,
            ]) {
                continue;
            }
            metadata.insert(k.clone(), v.clone());
        }
        let get_meta = |key: &str| {
            let key = String::from(crate::globals::RESERVED_METADATA_PREFIX_LOWER) + key;
            self.meta_sys
                .get(&key)
                .map(|v| v as &str)
                .unwrap_or_else(|| "")
                .to_owned()
        };
        use crate::bucket::*;
        Ok(crate::storage::FileInfo {
            volume: volume.to_string(),
            name: path.to_string(),
            version_id: self.version_id.map(|u| u.to_string()).unwrap_or_default(),
            is_latest: true,
            deleted: false,
            transition_status: get_meta(TRANSITION_STATUS),
            transition_object_name: get_meta(TRANSITIONED_OBJECT_NAME),
            transition_tier: get_meta(TRANSITIONED_VERSION_ID),
            transition_version_id: get_meta(TRANSITION_TIER),
            expire_restored: false,
            data_dir: self.data_dir.map(|u| u.to_string()).unwrap_or_default(),
            mod_time: utils::DateTime::from_timestamp_nanos(self.mod_time),
            size: self.size,
            mode: 0,
            metadata,
            parts,
            erasure: Some(erasure),
            mark_deleted: false,
            delete_marker_replication_status: "".to_string(),
            version_purge_status,
            data: vec![],
            num_versions: 0,
            successor_mod_time: utils::MIN_DATETIME,
        })
    }
}

impl XlMetaV2DeleteMarker {
    fn to_file_info(&self, volume: &str, path: &str) -> anyhow::Result<crate::storage::FileInfo> {
        let mut delete_marker_replication_status = None;
        let mut version_purge_status = None;
        for (k, v) in &self.meta_sys {
            if k.in_ignore_ascii_case(&[crate::storage::VERSION_PURGE_STATUS_KEY]) {
                version_purge_status = Some(crate::storage::VersionPurgeStatus::from_str(v)?);
            } else if k.in_ignore_ascii_case(&[crate::storage::VERSION_PURGE_STATUS_KEY]) {
                delete_marker_replication_status = Some(v.to_owned());
            }
        }

        Ok(crate::storage::FileInfo {
            volume: volume.to_string(),
            name: path.to_string(),
            version_id: self.version_id.map(|u| u.to_string()).unwrap_or_default(),
            is_latest: true,
            deleted: true,
            transition_status: "".to_string(),
            transition_object_name: "".to_string(),
            transition_tier: "".to_string(),
            transition_version_id: "".to_string(),
            expire_restored: false,
            data_dir: "".to_string(),
            mod_time: utils::DateTime::from_timestamp_nanos(self.mod_time),
            size: 0,
            mode: 0,
            metadata: Default::default(),
            parts: vec![],
            erasure: None,
            mark_deleted: false,
            delete_marker_replication_status: delete_marker_replication_status.unwrap_or_default(),
            version_purge_status: None,
            data: vec![],
            num_versions: 0,
            successor_mod_time: utils::MIN_DATETIME,
        })
    }
}

const XL_META_INLINE_DATA_VERSION: u8 = 1;

impl XlMetaV2 {
    pub fn add_version(&mut self, fi: &crate::storage::FileInfo) -> anyhow::Result<()> {
        let version_id: &str = if !fi.version_id.is_empty() {
            &fi.version_id
        } else {
            super::NULL_VERSION_ID
        };

        let mut uv = None;
        if version_id != super::NULL_VERSION_ID {
            uv = Some(uuid::Uuid::parse_str(version_id)?);
        }

        let mut dd = None;
        if !fi.data_dir.is_empty() {
            dd = Some(uuid::Uuid::parse_str(&fi.data_dir)?);
        }

        let version_entry = if fi.deleted {
            XlMetaV2Version {
                type_: VersionType::Delete,
                object_v2: None,
                delete_marker: Some(XlMetaV2DeleteMarker {
                    version_id: uv,
                    mod_time: fi.mod_time.timestamp_nanos(),
                    meta_sys: Default::default(),
                }),
            }
        } else {
            let erasure = fi.erasure.as_ref().unwrap();
            let mut version_entry = XlMetaV2Version {
                type_: VersionType::Object,
                object_v2: Some(XlMetaV2Object {
                    version_id: uv,
                    data_dir: dd,
                    erasure_algorithm: ErasureAlgo::ReedSolomon,
                    erasure_m: erasure.data_blocks,
                    erasure_n: erasure.parity_blocks,
                    erasure_block_size: erasure.block_size,
                    erasure_index: erasure.index,
                    erasure_distribution: erasure.distribution.clone(),
                    bitrot_checksum_algo: ChecksumAlgo::HighwayHash,
                    part_numbers: Vec::with_capacity(fi.parts.len()),
                    part_etags: Vec::with_capacity(fi.parts.len()),
                    part_sizes: Vec::with_capacity(fi.parts.len()),
                    part_actual_sizes: Vec::with_capacity(fi.parts.len()),
                    size: fi.size,
                    mod_time: fi.mod_time.timestamp_nanos(),
                    meta_sys: Default::default(),
                    meta_user: Default::default(),
                }),
                delete_marker: None,
            };

            let mut object_v2 = version_entry.object_v2.as_mut().unwrap();
            for part in fi.parts.iter() {
                object_v2.part_sizes.push(part.size);
                object_v2.part_etags.push(part.etag.clone());
                object_v2.part_numbers.push(part.number);
                object_v2.part_actual_sizes.push(part.actual_size);
            }

            for (k, v) in &fi.metadata {
                if k.to_lowercase()
                    .starts_with(crate::globals::RESERVED_METADATA_PREFIX_LOWER)
                {
                    object_v2.meta_sys.insert(k.to_owned(), v.to_owned());
                } else {
                    object_v2.meta_user.insert(k.to_owned(), v.to_owned());
                }
            }

            // If asked to save data.
            if !fi.data.is_empty() || fi.size == 0 {
                self.data.insert(version_id.to_owned(), fi.data.clone());
            }

            let mut insert = |key: &str, val: &str| {
                let key = String::from(crate::globals::RESERVED_METADATA_PREFIX_LOWER) + key;
                object_v2.meta_sys.insert(key, val.into());
            };
            use crate::bucket::*;
            if !fi.transition_status.is_empty() {
                insert(TRANSITION_STATUS, &fi.transition_status);
            }
            if !fi.transition_object_name.is_empty() {
                insert(TRANSITIONED_OBJECT_NAME, &fi.transition_object_name);
            }
            if !fi.transition_version_id.is_empty() {
                insert(TRANSITIONED_VERSION_ID, &fi.transition_version_id);
            }
            if !fi.transition_tier.is_empty() {
                insert(TRANSITION_TIER, &fi.transition_tier);
            }

            version_entry
        };

        if !version_entry.valid() {
            anyhow::bail!("generated invalid XlMetaV2Version");
        }

        for version in &mut self.versions {
            if !version.valid() {
                return Err(StorageError::FileCorrupt.into());
            }
            match version.type_ {
                VersionType::Object => {
                    if version.object_v2.as_ref().unwrap().version_id == uv {
                        *version = version_entry.clone();
                    }
                }
                VersionType::Delete => {
                    // Allowing delete marker to replaced with a proper
                    // object data type as well, this is not S3 complaint
                    // behavior but kept here for future flexibility.
                    if version.delete_marker.as_ref().unwrap().version_id == uv {
                        *version = version_entry.clone();
                    }
                }
            }
        }

        self.versions.push(version_entry);
        Ok(())
    }

    pub fn update_version(&mut self, fi: &crate::storage::FileInfo) -> anyhow::Result<()> {
        let version_id: &str = if !fi.version_id.is_empty() {
            &fi.version_id
        } else {
            super::NULL_VERSION_ID
        };

        let mut uv = None;
        if version_id != super::NULL_VERSION_ID {
            uv = Some(uuid::Uuid::parse_str(version_id)?);
        }

        for version in &mut self.versions {
            if !version.valid() {
                return Err(StorageError::FileCorrupt.into());
            }
            match version.type_ {
                VersionType::Object => {
                    if version.object_v2.as_ref().unwrap().version_id == uv {
                        let mut object_v2 = version.object_v2.as_mut().unwrap();
                        for (k, v) in &fi.metadata {
                            if k.to_lowercase()
                                .starts_with(crate::globals::RESERVED_METADATA_PREFIX_LOWER)
                            {
                                object_v2.meta_sys.insert(k.to_owned(), v.to_owned());
                            } else {
                                object_v2.meta_user.insert(k.to_owned(), v.to_owned());
                            }
                        }
                        if !fi.mod_time.is_min() {
                            object_v2.mod_time = fi.mod_time.timestamp_nanos();
                        }
                        return Ok(());
                    }
                }
                VersionType::Delete => {
                    return Err(crate::errors::TypedError::MethodNotAllowed.into());
                }
            }
        }

        return Err(StorageError::FileVersionNotFound.into());
    }

    pub fn list_versions(
        &self,
        volume: &str,
        path: &str,
    ) -> anyhow::Result<(Vec<crate::storage::FileInfo>, utils::DateTime)> {
        let mut versions = Vec::new();
        for version in &self.versions {
            if !version.valid() {
                return Err(StorageError::FileCorrupt.into());
            }
            let fi = match version.type_ {
                VersionType::Object => version
                    .object_v2
                    .as_ref()
                    .unwrap()
                    .to_file_info(volume, path)?,
                VersionType::Delete => version
                    .delete_marker
                    .as_ref()
                    .unwrap()
                    .to_file_info(volume, path)?,
            };
            versions.push(fi);
        }

        versions.sort_unstable_by(|a, b| {
            use std::cmp::Ordering;
            if a.is_latest {
                return Ordering::Less;
            };
            if b.is_latest {
                return Ordering::Greater;
            };
            a.mod_time.partial_cmp(&b.mod_time).unwrap()
        });

        for i in 0..versions.len() {
            versions[i].num_versions = versions.len();
            if i > 0 {
                versions[i].successor_mod_time = versions[i - 1].mod_time;
            }
        }

        versions[0].is_latest = true;

        let mod_time = versions[0].mod_time;
        Ok((versions, mod_time))
    }

    pub fn to_file_info(
        &self,
        volume: &str,
        path: &str,
        version_id: &str,
    ) -> anyhow::Result<crate::storage::FileInfo> {
        let uv = if !is_null_version_id(version_id) {
            Some(
                uuid::Uuid::parse_str(&version_id)
                    .map_err(|_| StorageError::FileVersionNotFound)?,
            )
        } else {
            None
        };

        for version in &self.versions {
            if !version.valid() {
                return if is_null_version_id(version_id) {
                    Err(StorageError::FileNotFound.into())
                } else {
                    Err(StorageError::FileVersionNotFound.into())
                };
            }
        }

        let mut versions: Vec<&XlMetaV2Version> = self.versions.iter().collect();
        versions.sort_unstable_by(|a, b| {
            let t1 = get_mod_time_from_version(a);
            let t2 = get_mod_time_from_version(b);
            t2.partial_cmp(&t1).unwrap()
        });

        if is_null_version_id(version_id) {
            if versions.is_empty() {
                return Err(StorageError::FileNotFound.into());
            }
            let version = versions[0];
            let mut fi = match version.type_ {
                VersionType::Object => version
                    .object_v2
                    .as_ref()
                    .unwrap()
                    .to_file_info(volume, path)?,
                VersionType::Delete => version
                    .delete_marker
                    .as_ref()
                    .unwrap()
                    .to_file_info(volume, path)?,
            };
            fi.is_latest = true;
            fi.num_versions = versions.len();
            return Ok(fi);
        }

        let mut fi = None;
        let mut index = 0;
        for (i, version) in versions.iter().enumerate() {
            match version.type_ {
                VersionType::Object => {
                    let object_v2 = version.object_v2.as_ref().unwrap();
                    if object_v2.version_id == uv {
                        fi = Some(object_v2.to_file_info(volume, path)?);
                        index = i;
                        break;
                    }
                }
                VersionType::Delete => {
                    let delete_marker = version.delete_marker.as_ref().unwrap();
                    if delete_marker.version_id == uv {
                        fi = Some(delete_marker.to_file_info(volume, path)?);
                        index = i;
                        break;
                    }
                }
            }
        }

        if let Some(mut fi) = fi {
            fi.is_latest = index == 0;
            fi.num_versions = versions.len();
            if index > 0 {
                fi.successor_mod_time = get_mod_time_from_version(versions[index - 1]);
            }
            return Ok(fi);
        }

        return if is_null_version_id(version_id) {
            Err(StorageError::FileNotFound.into())
        } else {
            Err(StorageError::FileVersionNotFound.into())
        };
    }

    pub fn delete_version(
        &mut self,
        fi: &crate::storage::FileInfo,
    ) -> anyhow::Result<(String, bool)> {
        let version_id = if !fi.version_id.is_empty() {
            Some(
                uuid::Uuid::parse_str(&fi.version_id)
                    .map_err(|_| StorageError::FileVersionNotFound)?,
            )
        } else {
            None
        };

        let mut ventry = None;
        if fi.deleted {
            ventry = Some(XlMetaV2Version {
                type_: VersionType::Delete,
                object_v2: None,
                delete_marker: Some(XlMetaV2DeleteMarker {
                    version_id,
                    mod_time: fi.mod_time.timestamp_nanos(),
                    meta_sys: Default::default(),
                }),
            });
            assert!(ventry.as_ref().unwrap().valid())
        }

        let mut update_version = false;
        if fi.version_purge_status.is_none()
            && (fi.delete_marker_replication_status == "REPLICA"
                || fi.delete_marker_replication_status.is_empty())
        {
            update_version = fi.mark_deleted;
        } else {
            if fi.deleted
                && fi.version_purge_status != Some(crate::storage::VersionPurgeStatus::Complete)
                && (fi.version_purge_status.is_some()
                    || !fi.delete_marker_replication_status.is_empty())
            {
                update_version = true;
            }
            if fi.version_purge_status.is_some()
                && fi.version_purge_status != Some(crate::storage::VersionPurgeStatus::Complete)
            {
                update_version = true;
            }
        }

        if fi.deleted {
            let delete_marker = ventry.as_mut().unwrap().delete_marker.as_mut().unwrap();
            if fi.delete_marker_replication_status.is_empty() {
                delete_marker.meta_sys.insert(
                    crate::http::AMZ_BUCKET_REPLICATION_STATUS.to_owned(),
                    fi.delete_marker_replication_status.clone(),
                );
            }
            if fi.version_purge_status.is_some() {
                delete_marker.meta_sys.insert(
                    crate::storage::VERSION_PURGE_STATUS_KEY.to_owned(),
                    fi.version_purge_status.unwrap().to_string(),
                );
            }
        }

        for (i, version) in self.versions.iter_mut().enumerate() {
            if !version.valid() {
                return Err(StorageError::FileCorrupt.into());
            }
            match version.type_ {
                VersionType::Object => {
                    let object_v2 = version.object_v2.as_mut().unwrap();
                    if object_v2.version_id == version_id && update_version {
                        object_v2.meta_sys.insert(
                            crate::storage::VERSION_PURGE_STATUS_KEY.to_owned(),
                            fi.version_purge_status
                                .map(|v| v.to_string())
                                .unwrap_or_default(),
                        );
                        return Ok(("".to_owned(), false));
                    }
                }
                VersionType::Delete => {
                    let delete_marker = version.delete_marker.as_mut().unwrap();
                    if delete_marker.version_id == version_id {
                        if update_version {
                            delete_marker
                                .meta_sys
                                .remove(crate::http::AMZ_BUCKET_REPLICATION_STATUS);
                            delete_marker
                                .meta_sys
                                .remove(crate::storage::VERSION_PURGE_STATUS_KEY);
                            if !fi.delete_marker_replication_status.is_empty() {
                                delete_marker.meta_sys.insert(
                                    crate::http::AMZ_BUCKET_REPLICATION_STATUS.to_owned(),
                                    fi.delete_marker_replication_status.clone(),
                                );
                            }
                            if fi.version_purge_status.is_some() {
                                delete_marker.meta_sys.insert(
                                    crate::storage::VERSION_PURGE_STATUS_KEY.to_owned(),
                                    fi.version_purge_status.unwrap().to_string(),
                                );
                            }
                        } else {
                            self.versions.remove(i);
                            if fi.mark_deleted
                                && (fi.version_purge_status.is_none()
                                    || fi.version_purge_status
                                        != Some(crate::storage::VersionPurgeStatus::Complete))
                            {
                                self.versions.push(ventry.unwrap()); // TODO: unwrap?
                            }
                        }
                        return Ok(("".to_owned(), false));
                    }
                }
            }
        }

        for (i, version) in self.versions.iter_mut().enumerate() {
            if !version.valid() {
                return Err(StorageError::FileCorrupt.into());
            }
            match version.type_ {
                VersionType::Object => {
                    let object_v2 = version.object_v2.as_mut().unwrap();
                    let data_dir = object_v2
                        .data_dir
                        .map(|u| u.to_string())
                        .unwrap_or_default();
                    if object_v2.version_id == version_id {
                        if fi.expire_restored {
                            let meta_user = &mut object_v2.meta_user;
                            meta_user.remove(crate::http::AMZ_RESTORE);
                            meta_user.remove(crate::http::AMZ_RESTORE_EXPIRY_DAYS);
                            meta_user.remove(crate::http::AMZ_RESTORE_REQUEST_DATE);
                        } else if fi.transition_status
                            == crate::bucket::TransitionStatus::Complete.to_string()
                        {
                            let mut insert = |key: &str, val: &str| {
                                let key =
                                    String::from(crate::globals::RESERVED_METADATA_PREFIX_LOWER)
                                        + key;
                                object_v2.meta_sys.insert(key, val.into());
                            };
                            use crate::bucket::*;
                            insert(TRANSITION_STATUS, &fi.transition_status);
                            insert(TRANSITIONED_OBJECT_NAME, &fi.transition_object_name);
                            insert(TRANSITIONED_VERSION_ID, &fi.transition_version_id);
                            insert(TRANSITION_TIER, &fi.transition_tier);
                        } else {
                            self.versions.remove(i);
                        }

                        if fi.deleted {
                            self.versions.push(ventry.unwrap());
                        }
                        if self.shared_data_dir_count(
                            &version_id,
                            &Some(uuid::Uuid::parse_str(&data_dir).unwrap()),
                        ) > 0
                        {
                            return Ok(("".to_owned(), false));
                        } else {
                            return Ok((data_dir, false));
                        }
                    }
                }
                _ => {}
            }
        }

        if fi.deleted {
            self.versions.push(ventry.unwrap());
            return Ok(("".to_owned(), false));
        }

        Err(StorageError::FileVersionNotFound.into())
    }

    pub fn shared_data_dir_str_count(&self, version_id: &str, data_dir: &str) -> usize {
        let version_id = if version_id == super::NULL_VERSION_ID {
            None
        } else {
            match uuid::Uuid::parse_str(version_id) {
                Ok(u) => Some(u),
                Err(_) => return 0,
            }
        };
        let data_dir = match uuid::Uuid::parse_str(data_dir) {
            Ok(u) => Some(u),
            Err(_) => return 0,
        };
        self.shared_data_dir_count(&version_id, &data_dir)
    }

    fn shared_data_dir_index_count(&self, index: usize) -> usize {
        let object_v2 = &self.versions[index].object_v2.as_ref().unwrap();
        self.shared_data_dir_count(&object_v2.version_id, &object_v2.data_dir)
    }

    fn shared_data_dir_count(
        &self,
        version_id: &Option<uuid::Uuid>,
        data_dir: &Option<uuid::Uuid>,
    ) -> usize {
        if self.data.contains_key(
            version_id
                .map(|u| u.to_string())
                .as_ref()
                .map(|s| s as &str)
                .unwrap_or(super::NULL_VERSION_ID),
        ) {
            return 0;
        }

        self.versions.iter().fold(0, |acc, v| {
            if let VersionType::Object = v.type_ {
                let object_v2 = v.object_v2.as_ref().unwrap();
                if &object_v2.version_id != version_id
                    && &object_v2.data_dir == data_dir
                    && object_v2.uses_data_dir()
                {
                    return acc + 1;
                }
            }
            acc
        })
    }

    pub fn total_size(&self) -> u64 {
        self.versions
            .iter()
            .filter_map(|version| match version.type_ {
                VersionType::Object => Some(version.object_v2.as_ref().unwrap().size),
                VersionType::Delete => None,
            })
            .fold(0, |acc, size| acc + size)
    }

    pub fn dump(&self) -> anyhow::Result<Vec<u8>> {
        // Estimate vec capacity.
        let mut cap = size_of_val(XL_HEADER)
            + size_of_val(&XL_VERSION_CURRENT)
            + size_of::<XlMetaV2>()
            + size_of::<u32>()
            + size_of::<u8>();
        for (k, v) in &self.data {
            cap += k.len() + v.len();
        }
        let mut buf = Vec::with_capacity(cap);

        buf.extend_from_slice(XL_HEADER);
        buf.extend_from_slice(&XL_VERSION_CURRENT[..]);

        let data_offset = buf.len();
        rmp_serde::encode::write(&mut buf, self)?;

        let crc = utils::xx_hash(&buf[data_offset..]);
        buf.write_u32::<LittleEndian>(crc as u32)?;

        buf.write_u8(XL_META_INLINE_DATA_VERSION)?;
        rmp_serde::encode::write(&mut buf, &self.data)?;

        Ok(buf)
    }

    pub fn load_with_data(buf: &[u8]) -> anyhow::Result<XlMetaV2> {
        let (buf, major, minor) = check_xl2_v1(buf)?;
        match major {
            XL_VERSION_MAJOR => match minor {
                XL_VERSION_MINOR => {
                    let cbuf = std::io::Cursor::new(buf);
                    let mut de = rmp_serde::decode::Deserializer::new(cbuf);
                    let mut meta: XlMetaV2 = serde::de::Deserialize::deserialize(&mut de)?;
                    let mut remaining = &buf[de.position() as usize..];

                    let got = utils::xx_hash(&buf[..de.position() as usize]);

                    let crc = remaining.read_u32::<LittleEndian>()?;
                    if got as u32 != crc {
                        anyhow::bail!("xl.meta: crc mismatch, want 0x{:x}, got 0x{:x}", crc, got);
                    }

                    let inline_data_version = remaining[0];
                    if inline_data_version != XL_META_INLINE_DATA_VERSION {
                        anyhow::bail!(
                            "xl.meta: unknown inline data version 0x{:x}",
                            inline_data_version
                        );
                    }
                    let data = &remaining[1..];

                    if !data.is_empty() {
                        meta.data = rmp_serde::from_read_ref(data)?;
                    }
                    return Ok(meta);
                }
                _ => {
                    anyhow::bail!("xl.meta: unknown minor metadata version");
                }
            },
            _ => {
                anyhow::bail!("xl.meta: unknown major metadata version");
            }
        }
    }

    pub async fn load_from_file(path: &str) -> anyhow::Result<XlMetaV2> {
        let file = crate::fs::StdOpenOptions::new()
            .read(true)
            .no_atime()
            .open(path)?;
        let mut reader = std::io::BufReader::with_capacity(4 << 10, file);
        tokio::task::spawn_blocking(move || Self::load_from_reader(&mut reader)).await?
    }

    fn load_from_reader<R: std::io::BufRead>(reader: &mut R) -> anyhow::Result<XlMetaV2> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        let (_, major, minor) = check_xl2_v1(&buf)?;
        match major {
            XL_VERSION_MAJOR => match minor {
                XL_VERSION_MINOR => {
                    let xx_reader = utils::XxHashReader::new(reader);

                    let mut de = rmp_serde::decode::Deserializer::new(xx_reader);
                    let meta: XlMetaV2 = serde::de::Deserialize::deserialize(&mut de)?;

                    let got = de.into_inner().hash();
                    let crc = reader.read_u32::<LittleEndian>()?;
                    if got as u32 != crc {
                        anyhow::bail!("xl.meta: crc mismatch, want 0x{:x}, got 0x{:x}", crc, got);
                    }

                    return Ok(meta);
                }
                _ => {
                    anyhow::bail!("xl.meta: unknown minor metadata version");
                }
            },
            _ => {
                anyhow::bail!("xl.meta: unknown major metadata version");
            }
        }
    }
}

fn get_mod_time_from_version(v: &XlMetaV2Version) -> utils::DateTime {
    match v.type_ {
        VersionType::Object => {
            utils::DateTime::from_timestamp_nanos(v.object_v2.as_ref().unwrap().mod_time)
        }
        VersionType::Delete => {
            utils::DateTime::from_timestamp_nanos(v.delete_marker.as_ref().unwrap().mod_time)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hash;
    use std::ptr::hash;

    use maplit::hashmap;

    use super::*;
    use crate::bitrot::BitrotAlgorithm;
    use crate::bucket::{RestoreStatus, TransitionStatus};
    use crate::storage::{FileInfo, VersionPurgeStatus};
    use crate::utils;
    use crate::utils::assert::{assert_err, assert_ok};
    use crate::utils::{now, ChronoDuration, DateTime};
    use crate::xl_storage::{ChecksumInfo, ErasureInfo};

    // xl_meta_v2_trim_data will trim any data from the metadata.
    // If any error occurs the unmodified data is returned.
    fn xl_meta_v2_trim_data(buf: &[u8]) -> Vec<u8> {
        let checked_buf = check_xl2_v1(buf);
        match checked_buf {
            Ok((meta_buf, major, minor)) => match major {
                XL_VERSION_MAJOR => match minor {
                    XL_VERSION_MINOR => {
                        let cbuf = std::io::Cursor::new(meta_buf);
                        let mut de = rmp_serde::decode::Deserializer::new(cbuf);
                        let _: XlMetaV2 = serde::de::Deserialize::deserialize(&mut de).unwrap();
                        let ends = buf.len()
                            - (meta_buf.len()
                                - (de.position() as usize + size_of::<u32>() + size_of::<u8>()));
                        buf[..ends].to_vec()
                    }
                    _ => buf.to_vec(),
                },
                _ => buf.to_vec(),
            },
            Err(_) => buf.to_vec(),
        }
    }

    #[test]
    fn test_xl_v2_format_data() {
        let data = b"some object data";
        let data2 = b"some other object data";
        let mut xl = XlMetaV2 {
            versions: Vec::new(),
            data: HashMap::new(),
        };
        let mut fi = FileInfo {
            volume: String::from("volume"),
            name: String::from("object-name"),
            version_id: String::from("756100c6-b393-4981-928a-d49bbc164741"),
            is_latest: true,
            deleted: false,
            transition_status: String::new(),
            transition_object_name: String::new(),
            transition_tier: String::new(),
            transition_version_id: String::new(),
            expire_restored: false,
            data_dir: String::from("bffea160-ca7f-465f-98bc-9b4f1c3ba1ef"),
            mod_time: utils::now(),
            size: 0,
            mode: 0,
            metadata: HashMap::new(),
            parts: Vec::new(),
            erasure: Some(ErasureInfo {
                algorithm: ErasureAlgo::ReedSolomon.to_string(),
                data_blocks: 4,
                parity_blocks: 2,
                block_size: 10000,
                index: 1,
                distribution: vec![1, 2, 3, 4, 5, 6, 7, 8],
                checksums: vec![ChecksumInfo {
                    part_number: 1,
                    algorithm: BitrotAlgorithm::HighwayHash256,
                    hash: Vec::new(),
                }],
            }),
            mark_deleted: false,
            delete_marker_replication_status: String::new(),
            version_purge_status: None,
            data: data.to_vec(),
            num_versions: 1,
            successor_mod_time: utils::MIN_DATETIME,
        };

        assert_ok!(xl.add_version(&fi));
        fi.version_id = uuid::Uuid::new_v4().to_string();
        fi.data_dir = uuid::Uuid::new_v4().to_string();
        fi.data = data2.to_vec();
        assert_ok!(xl.add_version(&fi));

        let serialized = assert_ok!(xl.dump());

        // Roundtrip data
        let mut xl2 = assert_ok!(XlMetaV2::load_with_data(&serialized));

        // We should have one data entry
        assert_eq!(xl2.data.len(), 2, "want 2 entry, got {}", xl2.data.len());
        assert_eq!(
            xl2.data.get("756100c6-b393-4981-928a-d49bbc164741"),
            Some(&data.to_vec()),
            "Find data returned {:?}",
            xl2.data
                .get("756100c6-b393-4981-928a-d49bbc164741")
                .unwrap()
        );
        assert_eq!(
            xl2.data.get(&fi.version_id),
            Some(&data2.to_vec()),
            "Find data returned {:?}",
            xl2.data.get(&fi.version_id).unwrap()
        );
        // Remove entry
        xl2.data.remove(&fi.version_id);
        assert_eq!(
            xl2.data.get(&fi.version_id),
            None,
            "Data was not removed: {:?}",
            xl2.data.get(&fi.version_id)
        );
        assert_eq!(xl2.data.len(), 1, "want 1 entry, got {}", xl2.data.len());

        // Re-add
        xl2.data.insert(fi.version_id.clone(), fi.data.clone());
        assert_eq!(xl2.data.len(), 2, "want 2 entry, got {}", xl2.data.len());

        // Replace entry
        xl2.data.insert(
            "756100c6-b393-4981-928a-d49bbc164741".to_string(),
            data2.to_vec(),
        );
        assert_eq!(xl2.data.len(), 2, "want 2 entry, got {}", xl2.data.len());
        assert_eq!(
            xl2.data.get("756100c6-b393-4981-928a-d49bbc164741"),
            Some(&data2.to_vec()),
            "Find data returned {:?}",
            xl2.data
                .get("756100c6-b393-4981-928a-d49bbc164741")
                .unwrap()
        );
        let remove_data = xl2
            .data
            .remove("756100c6-b393-4981-928a-d49bbc164741")
            .unwrap();
        xl2.data.insert("new-key".to_string(), remove_data);
        assert_eq!(
            xl2.data.get("new-key"),
            Some(&data2.to_vec()),
            "Find data returned {:?}",
            xl2.data
                .get("756100c6-b393-4981-928a-d49bbc164741")
                .unwrap()
        );
        assert_eq!(xl2.data.len(), 2, "want 2 entry, got {}", xl2.data.len());
        assert_eq!(
            xl2.data.get(&fi.version_id),
            Some(&data2.to_vec()),
            "Find data returned {:?}",
            xl2.data.get(&fi.version_id).unwrap()
        );

        // Test trimmed
        let mut trimmed = xl_meta_v2_trim_data(&serialized);
        let xl2 = assert_ok!(XlMetaV2::load_with_data(&trimmed));
        assert_eq!(
            xl2.data.len(),
            0,
            "data, was not trimmed, bytes left: {}",
            xl2.data.len()
        );
        // Corrupt metadata, last 5 bytes is the checksum, so go a bit further back.
        let trimmed_len = trimmed.len();
        if trimmed[trimmed_len - 5] < 128 {
            trimmed[trimmed_len - 5] += 10;
        } else {
            trimmed[trimmed_len - 5] -= 10;
        }
        assert_err!(
            XlMetaV2::load_with_data(&trimmed),
            "metadata corruption not detected"
        );
    }

    #[test]
    fn test_uses_data_dir() {
        let version_id = Some(uuid::Uuid::new_v4());
        let data_dir = Some(uuid::Uuid::new_v4());
        let transitioned = hashmap! {
            String::from(crate::globals::RESERVED_METADATA_PREFIX_LOWER)
                + crate::bucket::TRANSITION_STATUS =>
            crate::bucket::TransitionStatus::Complete.to_string()
        };
        let to_be_restored = hashmap! {
            String::from(crate::http::AMZ_RESTORE) =>
            RestoreStatus::ongoing().to_string()
        };

        let restored = hashmap! {
            String::from(crate::http::AMZ_RESTORE) =>
            RestoreStatus::completed(DateTime::from(now().checked_add_signed(ChronoDuration::hours(1)).unwrap())).to_string()
        };

        let restored_expired = hashmap! {
            String::from(crate::http::AMZ_RESTORE) =>
            RestoreStatus::completed(DateTime::from(now().checked_sub_signed(ChronoDuration::hours(1)).unwrap())).to_string()
        };
        let case_default = XlMetaV2Object {
            version_id: Some(uuid::Uuid::new_v4()),
            data_dir: Some(uuid::Uuid::new_v4()),
            meta_sys: HashMap::new(),
            meta_user: HashMap::new(),
            erasure_algorithm: ErasureAlgo::ReedSolomon,
            erasure_m: 0,
            erasure_n: 0,
            erasure_block_size: 0,
            erasure_index: 0,
            erasure_distribution: Vec::new(),
            bitrot_checksum_algo: ChecksumAlgo::HighwayHash,
            part_numbers: Vec::new(),
            part_etags: Vec::new(),
            part_sizes: Vec::new(),
            part_actual_sizes: Vec::new(),
            size: 0,
            mod_time: 0,
        };

        let cases: [(XlMetaV2Object, bool); 5] = [
            (
                // transitioned object version
                XlMetaV2Object {
                    version_id,
                    data_dir,
                    meta_sys: transitioned.clone(),
                    ..case_default.clone()
                },
                false,
            ),
            (
                // to be restored (requires object version to be transitioned)
                XlMetaV2Object {
                    version_id,
                    data_dir,
                    meta_sys: transitioned.clone(),
                    meta_user: to_be_restored.clone(),
                    ..case_default.clone()
                },
                false,
            ),
            (
                // restored object version (requires object version to be transitioned)
                XlMetaV2Object {
                    version_id,
                    data_dir,
                    meta_sys: transitioned.clone(),
                    meta_user: restored.clone(),
                    ..case_default.clone()
                },
                true,
            ),
            (
                // restored object version expired an hour back (requires object version to be transitioned)
                XlMetaV2Object {
                    version_id,
                    data_dir,
                    meta_sys: transitioned.clone(),
                    meta_user: restored_expired.clone(),
                    ..case_default.clone()
                },
                false,
            ),
            (
                // object version with no ILM applied
                XlMetaV2Object {
                    version_id,
                    data_dir,
                    ..case_default.clone()
                },
                true,
            ),
        ];
        for (i, (xmlmeta, uses)) in cases.iter().enumerate() {
            assert_eq!(xmlmeta.uses_data_dir(), *uses, "case {}", &i);
        }
    }

    #[test]
    fn test_delete_version_with_shared_data_dir() {
        let data = b"some object data";
        let data2 = b"some other object data";
        let mut xl = XlMetaV2 {
            versions: Vec::new(),
            data: HashMap::new(),
        };
        let d0 = uuid::Uuid::new_v4().to_string();
        let d1 = uuid::Uuid::new_v4().to_string();
        let d2 = uuid::Uuid::new_v4().to_string();
        let cases: [(String, String, &[u8], usize, String, String, bool, String); 7] = [
            (
                // object versions with inlined data don't count towards shared data directory
                uuid::Uuid::new_v4().to_string(),
                d0.clone(),
                data,
                0,
                String::new(),
                String::new(),
                false,
                String::new(),
            ),
            (
                // object versions with inlined data don't count towards shared data directory
                uuid::Uuid::new_v4().to_string(),
                d1.clone(),
                data2,
                0,
                String::new(),
                String::new(),
                false,
                String::new(),
            ),
            (
                // transitioned object version don't count towards shared data directory
                uuid::Uuid::new_v4().to_string(),
                d2.clone(),
                b"",
                3,
                TransitionStatus::Complete.to_string(),
                String::new(),
                false,
                String::new(),
            ),
            (
                // transitioned object version with an ongoing restore-object request.
                uuid::Uuid::new_v4().to_string(),
                d2.clone(),
                b"",
                3,
                TransitionStatus::Complete.to_string(),
                RestoreStatus::ongoing().to_string(),
                false,
                String::new(),
            ),
            // The following versions are on-disk.
            (
                // restored object version expiring 10 hours from now.
                uuid::Uuid::new_v4().to_string(),
                d2.clone(),
                b"",
                2,
                TransitionStatus::Complete.to_string(),
                RestoreStatus::completed(DateTime::from(
                    now().checked_add_signed(ChronoDuration::hours(10)).unwrap(),
                ))
                .to_string(),
                true,
                String::new(),
            ),
            (
                uuid::Uuid::new_v4().to_string(),
                d2.clone(),
                b"",
                2,
                String::new(),
                String::new(),
                false,
                String::new(),
            ),
            (
                uuid::Uuid::new_v4().to_string(),
                d2.clone(),
                b"",
                2,
                String::new(),
                String::new(),
                false,
                d2.clone(),
            ),
        ];
        let mut file_infos = Vec::new();
        for (
            version_id,
            data_dir,
            data,
            shares,
            transition_status,
            restore_obj_status,
            expire_restored,
            expected_data_dir,
        ) in cases.iter()
        {
            let mut fi = FileInfo {
                volume: String::from("volume"),
                name: String::from("object-name"),
                version_id: version_id.to_string(),
                is_latest: true,
                deleted: false,
                transition_status: String::new(),
                transition_object_name: String::new(),
                transition_tier: String::new(),
                transition_version_id: String::new(),
                expire_restored: false,
                data_dir: data_dir.to_string(),
                mod_time: utils::now(),
                size: 0,
                mode: 0,
                metadata: HashMap::new(),
                parts: Vec::new(),
                erasure: Some(ErasureInfo {
                    algorithm: ErasureAlgo::ReedSolomon.to_string(),
                    data_blocks: 4,
                    parity_blocks: 2,
                    block_size: 10000,
                    index: 1,
                    distribution: vec![1, 2, 3, 4, 5, 6, 7, 8],
                    checksums: vec![ChecksumInfo {
                        part_number: 1,
                        algorithm: BitrotAlgorithm::HighwayHash256,
                        hash: Vec::new(),
                    }],
                }),
                mark_deleted: false,
                delete_marker_replication_status: String::new(),
                version_purge_status: None,
                data: data.to_vec(),
                num_versions: 1,
                successor_mod_time: utils::MIN_DATETIME,
            };
            if fi.data.len() == 0 {
                fi.size = 42;
            }
            if restore_obj_status.len() > 0 {
                fi.metadata = hashmap! {
                    String::from(crate::http::AMZ_RESTORE) => restore_obj_status.to_string()
                }
            }
            fi.transition_status = transition_status.to_string();
            assert_ok!(xl.add_version(&fi));
            fi.expire_restored = *expire_restored;
            file_infos.insert(file_infos.len(), fi);
        }
        for (
            i,
            (
                version_id,
                data_dir,
                data,
                shares,
                transition_status,
                restore_obj_status,
                expire_restored,
                expected_data_dir,
            ),
        ) in cases.iter().enumerate()
        {
            let version = &xl.versions[i];
            assert_eq!(
                xl.shared_data_dir_count(
                    &version.object_v2.as_ref().unwrap().version_id,
                    &version.object_v2.as_ref().unwrap().data_dir
                ),
                *shares,
                "case {}",
                &i,
            );
        }
        // Deleting fileInfos[4].VersionID, fileInfos[5].VersionID should return empty data dir; there are other object version sharing the data dir.
        // Subsequently deleting fileInfos[6].versionID should return fileInfos[6].dataDir since there are no other object versions sharing this data dir.
        for (
            i,
            (
                version_id,
                data_dir,
                data,
                shares,
                transition_status,
                restore_obj_status,
                expire_restored,
                expected_data_dir,
            ),
        ) in cases[4..].iter().enumerate()
        {
            let del_data_dir = assert_ok!(xl.delete_version(file_infos.get(i + 4).unwrap()));
            assert_eq!(del_data_dir.0, *expected_data_dir, "case {}", &i);
        }
    }
}
