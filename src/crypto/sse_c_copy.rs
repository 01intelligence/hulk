use actix_http::header::HeaderMap;

pub const SSEC_COPY: SsecCopy = SsecCopy {};

pub struct SsecCopy {
}

impl SsecCopy {
    pub fn is_requested(&self, headers: &HeaderMap) -> bool {
        todo!()
    }
}