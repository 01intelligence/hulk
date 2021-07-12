use std::fmt;
use std::str::FromStr;

use actix_web::http::header::{ByteRangeSpec, Range};

use crate::errors::TypedError;

#[derive(Default, Debug)]
pub struct HttpRange(Option<Range>);

impl fmt::Display for HttpRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(ref r) => {
                write!(f, "{}", r)
            }
            None => {
                write!(f, "")
            }
        }
    }
}

impl HttpRange {
    pub fn get_length(&self, size: u64) -> Option<u64> {
        let (_, length) = self.get_offset_length(size)?;
        Some(length)
    }

    pub fn get_offset_length(&self, size: u64) -> Option<(u64, u64)> {
        match self.0 {
            Some(ref r) => {
                if let Range::Bytes(r) = r {
                    let (start, end) = r[0].to_satisfiable_range(size)?;
                    return Some((start, end - start + 1));
                }
                None
            }
            None => None,
        }
    }

    pub fn get_string(&self, size: u64) -> String {
        match self.get_offset_length(size) {
            Some((offset, length)) => {
                format!("{}-{}", offset, offset + length - 1)
            }
            None => "".to_owned(),
        }
    }
}

impl FromStr for HttpRange {
    type Err = TypedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = Range::from_str(s).map_err(|_| TypedError::InvalidRange)?;
        if let Range::Bytes(ref v) = r {
            if v.len() == 1 {
                return Ok(HttpRange(Some(r)));
            }
        }
        Err(TypedError::InvalidRange)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_header_range() {
        let cases = vec![
            ("bytes=0-", 0, 10),
            ("bytes=1-", 1, 9),
            ("bytes=0-9", 0, 10),
            ("bytes=1-10", 1, 9),
            ("bytes=1-1", 1, 1),
            ("bytes=2-5", 2, 4),
            ("bytes=-5", 5, 5),
            ("bytes=-1", 9, 1),
            ("bytes=-1000", 0, 10),
        ];
        for (spec, expected_offset, expected_length) in cases {
            let range = HttpRange::from_str(spec).unwrap();
            let (offset, length) = range.get_offset_length(10).unwrap();
            assert_eq!(offset, expected_offset);
            assert_eq!(length, expected_length);
        }

        let cases = vec![
            "bytes=-",
            "bytes==",
            "bytes==1-10",
            "bytes=",
            "bytes=aa",
            "aa",
            "",
            "bytes=1-10-",
            "bytes=1--10",
            "bytes=-1-10",
            "bytes=10-11,12-14", // Unsupported by S3 (valid in RFC)
        ];
        for spec in cases {
            assert_matches!(HttpRange::from_str(spec), Err(_));
        }

        let cases = vec!["bytes=10-10", "bytes=10-", "bytes=100-", "bytes=-0"];
        for spec in cases {
            let range = HttpRange::from_str(spec).unwrap();
            assert_matches!(range.get_offset_length(10), None);
        }
    }
}
