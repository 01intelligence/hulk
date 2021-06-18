mod boolfunc;
mod func;
mod jwt;
mod key;
mod name;
mod value;

pub use boolfunc::*;
pub use func::*;
pub use jwt::*;
pub use key::*;
pub use name::*;
pub use value::*;

pub(self) fn canonical_key(key: &str) -> String {
    // todo: golang http.CanonicalHeaderKey
    key.to_owned()
}
