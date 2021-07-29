use actix_web::HttpRequest;

#[derive(Eq, PartialEq)]
pub enum AuthType {
    Unknown,
    Anonymous,
    Presigned,
    PresignedV2,
    PostPolicy,
    StreamingSigned,
    Signed,
    SignedV2,
    JWT,
    STS,
}

pub fn get_request_auth_type(req: &HttpRequest) -> AuthType {
    use AuthType::*;
    // TODO
    Unknown
}
