use super::*;

pub fn sign_with_standard_claims(claims: &StandardClaims, secret: &str) -> anyhow::Result<String> {
    Ok(jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS512),
        claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_ref()),
    )?)
}
