use std::borrow::Cow;

use actix_web::HttpRequest;

use crate::globals::{self, Guard, GLOBALS};

// Returns "/<bucket>/<object>" for path-style or virtual-host-style requests.
pub fn get_resource<'a>(
    path: &'a str,
    host: &str,
    domains: &[impl AsRef<str>],
) -> anyhow::Result<Cow<'a, str>> {
    let mut host = Cow::Borrowed(host);
    if domains.is_empty() {
        return Ok(Cow::Borrowed(path));
    }
    if host.contains(':') {
        host = Cow::Owned(crate::endpoint::split_host_port(host.as_ref())?.0);
    }
    let host = host.as_ref();
    for domain in domains {
        let domain = domain.as_ref();
        if host == &format!("{}.{}", globals::SYSTEM_RESERVED_BUCKET, domain) {
            continue;
        }
        if let Some(bucket) = host.strip_suffix(&format!("{}.{}", host, domain)) {
            return Ok(Cow::Owned(format!(
                "{}{}",
                globals::SLASH_SEPARATOR,
                crate::object::path_join(&[bucket, path])
            )));
        }
    }
    Ok(Cow::Borrowed(path))
}

pub fn request_to_bucket_object(req: &HttpRequest) -> (Cow<'_, str>, Cow<'_, str>) {
    let path = get_resource(
        req.path(),
        req.uri().host().unwrap(),
        &(*GLOBALS.domain_names.guard())[..],
    )
    .unwrap(); // TODO: unwrap?
    match path {
        Cow::Borrowed(path) => {
            let (bucket, object) = path_to_bucket_object(path);
            (bucket.into(), object.into())
        }
        Cow::Owned(path) => {
            let (bucket, object) = path_to_bucket_object(path.as_str());
            (bucket.to_owned().into(), object.to_owned().into())
        }
    }
}

pub fn path_to_bucket_object(path: &str) -> (&str, &str) {
    path_to_bucket_object_with_base_path("", path)
}

pub fn path_to_bucket_object_with_base_path<'a>(
    base_path: &str,
    path: &'a str,
) -> (&'a str, &'a str) {
    let path = path.strip_prefix(base_path).unwrap_or(path);
    let path = path.strip_prefix(globals::SLASH_SEPARATOR).unwrap_or(path);
    let mut splits = path.splitn(2, globals::SLASH_SEPARATOR);
    let path = splits.next().unwrap();
    (path, splits.next().unwrap_or(""))
}
