use actix_web::http::{header, Method};
use actix_web::HttpRequest;

use super::*;
use crate::http::{self, RequestExtensionsContext};

fn is_request_jwt(req: &HttpRequest) -> bool {
    req.headers()
        .get(header::AUTHORIZATION)
        .map(|h| {
            h.to_str()
                .map(|s| s.starts_with(JWT_ALGORITHM))
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn is_request_sign_v4(req: &HttpRequest) -> bool {
    req.headers()
        .get(header::AUTHORIZATION)
        .map(|h| {
            h.to_str()
                .map(|s| s.starts_with(SIGN_V4_ALGORITHM))
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn is_request_sign_v2(req: &HttpRequest) -> bool {
    req.headers()
        .get(header::AUTHORIZATION)
        .map(|h| {
            h.to_str()
                .map(|s| !s.starts_with(SIGN_V4_ALGORITHM) && s.starts_with(SIGN_V2_ALGORITHM))
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn is_request_presigned_sign_v4(req: &HttpRequest) -> bool {
    req.query()
        .as_ref()
        .map(|q| q.contains_key(http::AMZ_CREDENTIAL))
        .unwrap_or_default()
}

fn is_request_presigned_sign_v2(req: &HttpRequest) -> bool {
    req.query()
        .as_ref()
        .map(|q| q.contains_key(http::AMZ_ACCESS_KEY_ID))
        .unwrap_or_default()
}

fn is_request_post_policy_sign_v4(req: &HttpRequest) -> bool {
    req.method() == Method::POST
        && req
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|h| {
                h.to_str()
                    .map(|s| s.contains(mime::MULTIPART_FORM_DATA.as_ref()))
                    .unwrap_or_default()
            })
            .unwrap_or_default()
}

fn is_request_sign_streaming_v4(req: &HttpRequest) -> bool {
    req.method() == Method::PUT
        && req
            .headers()
            .get(http::AMZ_CONTENT_SHA256)
            .map(|h| {
                h.to_str()
                    .map(|s| s == STREAMING_CONTENT_SHA256)
                    .unwrap_or_default()
            })
            .unwrap_or_default()
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum AuthType {
    Unknown,
    Anonymous,
    Presigned,
    PresignedV2,
    PostPolicy,
    StreamingSigned,
    Signed,
    SignedV2,
    Jwt,
    Sts,
}

pub fn get_request_auth_type(req: &HttpRequest) -> AuthType {
    use AuthType::*;
    if is_request_sign_v2(req) {
        SignedV2
    } else if is_request_presigned_sign_v2(req) {
        PresignedV2
    } else if is_request_sign_streaming_v4(req) {
        StreamingSigned
    } else if is_request_sign_v4(req) {
        Signed
    } else if is_request_presigned_sign_v4(req) {
        Presigned
    } else if is_request_jwt(req) {
        Jwt
    } else if is_request_post_policy_sign_v4(req) {
        PostPolicy
    } else {
        if req
            .query()
            .as_ref()
            .map(|q| q.contains_key(http::ACTION))
            .unwrap_or_default()
        {
            Sts
        } else if req.headers().get(header::AUTHORIZATION).is_none() {
            Anonymous
        } else {
            Unknown
        }
    }
}

pub fn is_supported_s3_auth_type(auth_type: AuthType) -> bool {
    use AuthType::*;
    matches!(
        auth_type,
        Anonymous | Presigned | PresignedV2 | Signed | SignedV2 | PostPolicy | StreamingSigned
    )
}
