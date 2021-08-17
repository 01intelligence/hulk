use std::collections::HashMap;

use lazy_static::lazy_static;

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
        use byteorder::{LittleEndian, WriteBytesExt};
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
        use byteorder::{LittleEndian, ReadBytesExt};
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

#[repr(u8)]
pub enum VersionType {
    Object = 1,
    Delete = 2,
}

#[repr(u8)]
pub enum ErasureAlgo {
    ReedSolomon = 1,
}

#[repr(u8)]
pub enum ChecksumAlgo {
    HighwayHash = 1,
}

struct XlMetaV2DeleteMarker {
    version_id: [u8; 16],
    mod_time: i64,
    meta_sys: HashMap<String, Vec<u8>>, // internal metadata
}

struct XlMetaV2Object {
    version_id: [u8; 16],
    data_dir: [u8; 16],
    erasure_algorithm: ErasureAlgo,
    erasure_m: isize,
    erasure_n: isize,
    erasure_block_size: i64,
    erasure_index: isize,
    erasure_distribution: Vec<u8>,
    bitrot_checksum_algo: ChecksumAlgo,
    part_numbers: Vec<isize>,
    part_etags: Vec<String>,
    part_sizes: Vec<i64>,
    part_actual_sizes: Vec<i64>,
    size: i64,
    mod_time: i64,
    meta_sys: HashMap<String, Vec<u8>>, // internal metadata
    meta_user: HashMap<String, String>, // metadata set by user
}

struct XlMetaV2Version {
    type_: VersionType,
    object_v2: Option<XlMetaV2Object>,
    delete_marker: Option<XlMetaV2DeleteMarker>,
}

struct XlMetaV2 {
    versions: Vec<XlMetaV2Version>,
    data: XlMetaInlineData,
}

struct XlMetaInlineData(Vec<u8>);
