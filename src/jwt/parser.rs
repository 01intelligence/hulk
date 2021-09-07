use jsonwebtoken::{decode, decode_with_key_fn, Algorithm, DecodingKey, Validation};

use super::{JwtError, MapClaims, StandardClaims};

const ALGORITHMS: &[Algorithm] = &[Algorithm::HS256, Algorithm::HS384, Algorithm::HS512];

pub fn parse_with_standard_claims(token: &str, key: &[u8]) -> anyhow::Result<StandardClaims> {
    let validation = Validation {
        algorithms: ALGORITHMS.into(),
        ..Default::default()
    };

    let claims = decode::<StandardClaims>(token, &DecodingKey::from_secret(key), &validation)?;
    let claims = claims.claims;

    claims.validate()?;

    Ok(claims)
}

pub fn parse_with_claims<F>(token: &str, key_fn: F) -> anyhow::Result<MapClaims>
where
    F: FnOnce(&MapClaims) -> DecodingKey,
{
    let validation = Validation {
        algorithms: ALGORITHMS.into(),
        validate_exp: true,
        validate_iat: true,
        validate_nbf: true,
        ..Default::default()
    };

    let claims = decode_with_key_fn::<MapClaims, F>(token, key_fn, &validation)?;
    let claims = claims.claims;

    if claims.lookup("accessKey").is_null() && claims.lookup("sub").is_null() {
        return Err(JwtError::Other("accessKey/sub missing".into()).into());
    }

    Ok(claims)
}
