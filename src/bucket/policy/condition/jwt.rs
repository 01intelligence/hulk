use lazy_static::lazy_static;

use super::Key;

// JWT claims supported substitutions.
// https://www.iana.org/assignments/jwt/jwt.xhtml#claims

// Subject claim substitution.
pub const JWT_SUB: Key = Key("jwt:sub");

// Issuer claim substitution.
pub const JWT_ISS: Key = Key("jwt:iss");

// Audience claim substitution.
pub const JWT_AUD: Key = Key("jwt:aud");

// Unique identifier claim substitution.
pub const JWT_JTI: Key = Key("jwt:jti");

pub const JWT_UPN: Key = Key("jwt:upn");
pub const JWT_NAME: Key = Key("jwt:name");
pub const JWT_GROUPS: Key = Key("jwt:groups");
pub const JWT_GIVEN_NAME: Key = Key("jwt:given_name");
pub const JWT_FAMILY_NAME: Key = Key("jwt:family_name");
pub const JWT_MIDDLE_NAME: Key = Key("jwt:middle_name");
pub const JWT_NICK_NAME: Key = Key("jwt:nickname");
pub const JWT_PREF_USERNAME: Key = Key("jwt:preferred_username");
pub const JWT_PROFILE: Key = Key("jwt:profile");
pub const JWT_PICTURE: Key = Key("jwt:picture");
pub const JWT_WEBSITE: Key = Key("jwt:website");
pub const JWT_EMAIL: Key = Key("jwt:email");
pub const JWT_GENDER: Key = Key("jwt:gender");
pub const JWT_BIRTHDATE: Key = Key("jwt:birthdate");
pub const JWT_PHONE_NUMBER: Key = Key("jwt:phone_number");
pub const JWT_ADDRESS: Key = Key("jwt:address");
pub const JWT_SCOPE: Key = Key("jwt:scope");
pub const JWT_CLIENT_ID: Key = Key("jwt:client_id");

// Supported JWT keys, non-exhaustive list please
// expand as new claims are standardized.
lazy_static! {
    pub static ref JWT_KEYS: Vec<Key<'static>> = vec![
        JWT_SUB,
        JWT_ISS,
        JWT_AUD,
        JWT_JTI,
        JWT_UPN,
        JWT_NAME,
        JWT_GROUPS,
        JWT_GIVEN_NAME,
        JWT_FAMILY_NAME,
        JWT_MIDDLE_NAME,
        JWT_NICK_NAME,
        JWT_PREF_USERNAME,
        JWT_PROFILE,
        JWT_PICTURE,
        JWT_WEBSITE,
        JWT_EMAIL,
        JWT_GENDER,
        JWT_BIRTHDATE,
        JWT_PHONE_NUMBER,
        JWT_ADDRESS,
        JWT_SCOPE,
        JWT_CLIENT_ID,
    ];
}
