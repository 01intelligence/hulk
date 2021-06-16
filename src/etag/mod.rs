// Package etag provides an implementation of S3 ETags.
//
// Each S3 object has an associated ETag that can be
// used to e.g. quickly compare objects or check whether
// the content of an object has changed.
//
// In general, an S3 ETag is an MD5 checksum of the object
// content. However, there are many exceptions to this rule.
//
//
// Single-part Upload
//
// In case of a basic single-part PUT operation - without server
// side encryption or object compression - the ETag of an object
// is its content MD5.
//
//
// Multi-part Upload
//
// The ETag of an object does not correspond to its content MD5
// when the object is uploaded in multiple parts via the S3
// multipart API. Instead, S3 first computes a MD5 of each part:
//   e1 := MD5(part-1)
//   e2 := MD5(part-2)
//  ...
//   eN := MD5(part-N)
//
// Then, the ETag of the object is computed as MD5 of all individual
// part checksums. S3 also encodes the number of parts into the ETag
// by appending a -<number-of-parts> at the end:
//   ETag := MD5(e1 || e2 || e3 ... || eN) || -N
//
//   For example: ceb8853ddc5086cc4ab9e149f8f09c88-5
//
// However, this scheme is only used for multipart objects that are
// not encrypted.
//
// Server-side Encryption
//
// S3 specifies three types of server-side-encryption - SSE-C, SSE-S3
// and SSE-KMS - with different semantics w.r.t. ETags.
// In case of SSE-S3, the ETag of an object is computed the same as
// for single resp. multipart plaintext objects. In particular,
// the ETag of a singlepart SSE-S3 object is its content MD5.
//
// In case of SSE-C and SSE-KMS, the ETag of an object is computed
// differently. For singlepart uploads the ETag is not the content
// MD5 of the object. For multipart uploads the ETag is also not
// the MD5 of the individual part checksums but it still contains
// the number of parts as suffix.
//
// Instead, the ETag is kind of unpredictable for S3 clients when
// an object is encrypted using SSE-C or SSE-KMS. Maybe AWS S3
// computes the ETag as MD5 of the encrypted content but there is
// no way to verify this assumption since the encryption happens
// inside AWS S3.
// Therefore, S3 clients must not make any assumption about ETags
// in case of SSE-C or SSE-KMS except that the ETag is well-formed.
//
// To put all of this into a simple rule:
//    SSE-S3 : ETag == MD5
//    SSE-C  : ETag != MD5
//    SSE-KMS: ETag != MD5
//
//
// Encrypted ETags
//
// An S3 implementation has to remember the content MD5 of objects
// in case of SSE-S3. However, storing the ETag of an encrypted
// object in plaintext may reveal some information about the object.
// For example, two objects with the same ETag are identical with
// a very high probability.
//
// Therefore, an S3 implementation may encrypt an ETag before storing
// it. In this case, the stored ETag may not be a well-formed S3 ETag.
// For example, it can be larger due to a checksum added by authenticated
// encryption schemes. Such an ETag must be decrypted before sent to an
// S3 client.
//
//
// S3 Clients
//
// There are many different S3 client implementations. Most of them
// access the ETag by looking for the HTTP response header key "Etag".
// However, some of them assume that the header key has to be "ETag"
// (case-sensitive) and will fail otherwise.
// Further, some clients require that the ETag value is a double-quoted
// string. Therefore, this package provides dedicated functions for
// adding and extracing the ETag to/from HTTP headers.

mod reader;

use actix_web::web;
use anyhow::bail;
use hex;
pub use reader::*;

// ETag is a single S3 ETag.
//
// An S3 ETag sometimes corresponds to the MD5 of
// the S3 object content. However, when an object
// is encrypted, compressed or uploaded using
// the S3 multipart API then its ETag is not
// necessarily the MD5 of the object content.
#[derive(Eq, PartialEq)]
pub struct ETag(Vec<u8>);

impl ETag {
    // Parses s as an S3 ETag, returning the result.
    // The string can be an encrypted, singlepart
    // or multipart S3 ETag. It returns an error if s is
    // not a valid textual representation of an ETag.
    pub fn parse(s: &str) -> anyhow::Result<ETag> {
        parse(s, false)
    }

    // Decodes and returns the Content-MD5
    // as ETag, if set. If no Content-MD5 header is set
    // it returns an empty ETag and no error.
    pub fn from_content_md5(req: &web::HttpRequest) -> anyhow::Result<ETag> {
        let v = req.headers().get("Content-Md5");
        let v = if let Some(v) = v {
            v.to_str()?
        } else {
            return Ok(ETag(Vec::new()));
        };
        if v.is_empty() {
            bail!("etag content-md5 is set but contains no value");
        }
        let etag = base64::decode_config(v, base64::STANDARD)?;
        if etag.len() != 16 {
            bail!("etag invalid content-md5");
        }
        Ok(ETag(etag))
    }

    // Reports whether the ETag is encrypted.
    pub fn is_encrypted(&self) -> bool {
        self.0.len() > 16 && !self.0.contains(&b'-')
    }

    // Reports whether the ETag belongs to an
    // object that has been uploaded using the S3 multipart
    // API.
    // An S3 multipart ETag has a -<part-number> suffix.
    pub fn is_multipart(&self) -> bool {
        self.0.len() > 16 && self.0.contains(&b'-')
    }

    // Returns the number of object parts that are
    // referenced by this ETag. It returns 1 if the object
    // has been uploaded using the S3 singlepart API.
    //
    // May panic if the ETag is an invalid multipart
    // ETag.
    pub fn parts(&self) -> usize {
        if !self.is_multipart() {
            return 1;
        }

        let n = self.0.iter().position(|&c| c == b'-').unwrap();
        let parts = std::str::from_utf8(&self.0[n + 1..]).unwrap();
        parts.parse().unwrap()
    }
}

impl ToString for ETag {
    // Returns the string representation of the ETag.
    //
    // The returned string is a hex representation of the
    // binary ETag with an optional '-<part-number>' suffix.
    fn to_string(&self) -> String {
        if self.is_multipart() {
            hex::encode(&self.0[..16]) + unsafe { std::str::from_utf8_unchecked(&self.0[16..]) }
        } else {
            hex::encode(&self.0)
        }
    }
}

// Parse s as an S3 ETag, returning the result.
// It operates in one of two modes:
//  - strict
//  - non-strict
//
// In strict mode, parse only accepts ETags that
// are AWS S3 compatible. In particular, an AWS
// S3 ETag always consists of a 128 bit checksum
// value and an optional -<part-number> suffix.
// Therefore, s must have the following form in
// strict mode:  <32-hex-characters>[-<integer>]
//
// In non-strict mode, parse also accepts ETags
// that are not AWS S3 compatible - e.g. encrypted
// ETags.
fn parse(mut s: &str, strict: bool) -> anyhow::Result<ETag> {
    // An S3 ETag may be a double-quoted string.
    // Therefore, we remove double quotes at the
    // start and end, if any.
    if s.starts_with('"') && s.ends_with('"') {
        s = &s[1..s.len() - 1];
    }

    // An S3 ETag may be a multipart ETag that
    // contains a '-' followed by a number.
    // If the ETag does not a '-' is is either
    // a singlepart or encrypted ETag.
    let n = match s.find('-') {
        None => {
            let etag = hex::decode(s)?;
            if strict && etag.len() != 16 {
                // AWS S3 ETags are always 128 bit long
                bail!("etag invalid length {}", etag.len());
            }
            return Ok(ETag(etag));
        }
        Some(n) => n,
    };

    let (prefix, suffix) = s.split_at(n);
    if prefix.len() != 32 {
        bail!("etag invalid prefix length {}", prefix.len());
    }
    if suffix.len() <= 1 {
        bail!("etag suffix is not a part number");
    }

    let mut etag = hex::decode(prefix)?;
    // suffix[0] == '-' therefore we start parsing at suffix[1]
    let parts: usize = suffix[1..].parse()?;
    if strict && (parts == 0 || parts > 10000) {
        bail!("etag invalid part number {}", parts);
    }

    etag.extend_from_slice(suffix.as_bytes());
    Ok(ETag(etag))
}
