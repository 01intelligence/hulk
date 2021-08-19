use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::Display;
use uuid::{Error, Uuid};

use super::StorageError;
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
        (&mut version[..])
            .write_u16::<LittleEndian>(XL_VERSION_MAJOR)
            .unwrap();
        (&mut version[..])
            .write_u16::<LittleEndian>(XL_VERSION_MINOR)
            .unwrap();
        version
    };
}

fn check_xl2_v1(buf: &[u8]) -> anyhow::Result<(&[u8], u16, u16)> {
    if buf.len() <= 8 {
        anyhow::bail!("xl.meta: no data");
    }
    if &buf[..4] == &XL_HEADER[..] {
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

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy)]
#[repr(u8)]
pub enum VersionType {
    Object = 1,
    Delete = 2,
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, Display)]
#[repr(u8)]
pub enum ErasureAlgo {
    #[strum(serialize = "reedsolomn")]
    ReedSolomon = 1,
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy)]
#[repr(u8)]
pub enum ChecksumAlgo {
    HighwayHash = 1,
}

#[derive(Serialize, Deserialize, Clone)]
struct XlMetaV2DeleteMarker {
    #[serde(rename = "ID")]
    version_id: Option<uuid::Uuid>,
    #[serde(rename = "MTime")]
    mod_time: i64,
    #[serde(rename = "MetaSys", skip_serializing_if = "HashMap::is_empty")]
    meta_sys: HashMap<String, String>, // internal metadata
}

#[derive(Serialize, Deserialize, Clone)]
struct XlMetaV2Object {
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
    part_numbers: Vec<isize>,
    #[serde(rename = "PartETags")]
    part_etags: Vec<String>,
    #[serde(rename = "PartSizes")]
    part_sizes: Vec<i64>,
    #[serde(rename = "PartASizes", skip_serializing_if = "Vec::is_empty")]
    part_actual_sizes: Vec<i64>,
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "MTime")]
    mod_time: i64,
    #[serde(rename = "MetaSys", skip_serializing_if = "HashMap::is_empty")]
    meta_sys: HashMap<String, String>,
    // internal metadata
    #[serde(rename = "MetaUsr", skip_serializing_if = "HashMap::is_empty")]
    meta_user: HashMap<String, String>, // metadata set by user
}

#[derive(Serialize, Deserialize, Clone)]
struct XlMetaV2Version {
    #[serde(rename = "Type")]
    type_: VersionType,
    #[serde(rename = "V2Obj", skip_serializing_if = "Option::is_none")]
    object_v2: Option<XlMetaV2Object>,
    #[serde(rename = "DelObj", skip_serializing_if = "Option::is_none")]
    delete_marker: Option<XlMetaV2DeleteMarker>,
}

#[derive(Serialize, Deserialize)]
struct XlMetaV2 {
    #[serde(rename = "Versions")]
    versions: Vec<XlMetaV2Version>,
    #[serde(skip)]
    data: HashMap<String, Vec<u8>>,
}

struct XlMetaInlineData<'a>(Cow<'a, [u8]>);

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
            == self
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

impl<'a> XlMetaInlineData<'a> {
    fn version_ok(&self) -> bool {
        self.0.is_empty() || (self.0[0] > 0 && self.0[0] <= XL_META_INLINE_DATA_VERSION)
    }

    fn after_version(&self) -> &[u8] {
        if self.0.is_empty() {
            &self.0[..]
        } else {
            &self.0[1..]
        }
    }
}

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
            if version.valid() {
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
            if version.valid() {
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

                        if self.shared_data_dir_index_count(i) > 0 {
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

    pub fn load(buf: &[u8]) -> anyhow::Result<XlMetaV2> {
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
                    let remaining = &remaining[size_of::<u32>()..];

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
}
