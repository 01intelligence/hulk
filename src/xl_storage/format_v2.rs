use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::{size_of, size_of_val};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::StorageError;

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

#[derive(Serialize_repr, Deserialize_repr, Clone, Copy)]
#[repr(u8)]
pub enum ErasureAlgo {
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
    #[serde(rename = "MetaSys", skip_serializing_if = "Option::is_none")]
    meta_sys: Option<HashMap<String, Vec<u8>>>, // internal metadata
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
    #[serde(rename = "MetaSys", skip_serializing_if = "Option::is_none")]
    meta_sys: Option<HashMap<String, Vec<u8>>>, // internal metadata
    #[serde(rename = "MetaUsr", skip_serializing_if = "Option::is_none")]
    meta_user: Option<HashMap<String, String>>, // metadata set by user
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
    data: Option<HashMap<String, Vec<u8>>>,
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
                    meta_sys: None,
                }),
            }
        } else {
            let mut version_entry = XlMetaV2Version {
                type_: VersionType::Object,
                object_v2: Some(XlMetaV2Object {
                    version_id: uv,
                    data_dir: dd,
                    erasure_algorithm: ErasureAlgo::ReedSolomon,
                    erasure_m: fi.erasure.data_blocks,
                    erasure_n: fi.erasure.parity_blocks,
                    erasure_block_size: fi.erasure.block_size,
                    erasure_index: fi.erasure.index,
                    erasure_distribution: fi.erasure.distribution.clone(),
                    bitrot_checksum_algo: ChecksumAlgo::HighwayHash,
                    part_numbers: Vec::with_capacity(fi.parts.len()),
                    part_etags: Vec::with_capacity(fi.parts.len()),
                    part_sizes: Vec::with_capacity(fi.parts.len()),
                    part_actual_sizes: Vec::with_capacity(fi.parts.len()),
                    size: fi.size,
                    mod_time: fi.mod_time.timestamp_nanos(),
                    meta_sys: None,
                    meta_user: None,
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
                    object_v2
                        .meta_sys
                        .get_or_insert_default()
                        .insert(k.to_owned(), Vec::from(v.as_bytes()));
                } else {
                    object_v2
                        .meta_user
                        .get_or_insert_default()
                        .insert(k.to_owned(), v.to_owned());
                }
            }

            // If asked to save data.
            if !fi.data.is_empty() || fi.size == 0 {
                self.data
                    .get_or_insert_default()
                    .insert(version_id.to_owned(), fi.data.clone());
            }

            let mut insert = |key: &str, val: &str| {
                let key = String::from(crate::globals::RESERVED_METADATA_PREFIX_LOWER) + key;
                object_v2
                    .meta_sys
                    .get_or_insert_default()
                    .insert(key, val.into());
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
                                object_v2
                                    .meta_sys
                                    .get_or_insert_default()
                                    .insert(k.to_owned(), Vec::from(v.as_bytes()));
                            } else {
                                object_v2
                                    .meta_user
                                    .get_or_insert_default()
                                    .insert(k.to_owned(), v.to_owned());
                            }
                        }
                        if fi.mod_time != crate::utils::MIN_DATETIME {
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

    pub fn dump(&self) -> anyhow::Result<Vec<u8>> {
        // Estimate vec capacity.
        let mut cap = size_of_val(XL_HEADER)
            + size_of_val(&XL_VERSION_CURRENT)
            + size_of::<XlMetaV2>()
            + size_of::<u32>()
            + size_of::<u8>();
        if let Some(data) = &self.data {
            for (k, v) in data {
                cap += k.len() + v.len();
            }
        }
        let mut buf = Vec::with_capacity(cap);

        buf.extend_from_slice(XL_HEADER);
        buf.extend_from_slice(&XL_VERSION_CURRENT[..]);

        let data_offset = buf.len();
        rmp_serde::encode::write(&mut buf, self)?;

        let crc = crate::utils::xx_hash(&buf[data_offset..]);
        buf.write_u32::<LittleEndian>(crc as u32)?;

        buf.write_u8(XL_META_INLINE_DATA_VERSION)?;
        if let Some(data) = &self.data {
            rmp_serde::encode::write(&mut buf, data)?;
        }

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

                    let got = crate::utils::xx_hash(&buf[..de.position() as usize]);

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
                        meta.data = Some(rmp_serde::from_read_ref(data)?);
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
