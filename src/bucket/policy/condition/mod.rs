mod binaryequalsfunc;
mod boolfunc;
mod func;
mod jwt;
mod key;
mod name;
mod stringequalsfunc;
mod stringequalsignorecasefunc;
mod stringlikefunc;
mod value;

pub use binaryequalsfunc::*;
pub use boolfunc::*;
pub use func::*;
pub use jwt::*;
pub use key::*;
pub use name::*;
pub use stringequalsfunc::*;
pub use stringequalsignorecasefunc::*;
pub use stringlikefunc::*;
pub use value::*;

pub(self) fn canonical_key(key: &str) -> String {
    // todo: golang http.CanonicalHeaderKey
    key.to_owned()
}