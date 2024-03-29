mod binaryequalsfunc;
mod boolfunc;
mod dateequalsfunc;
mod dategreaterthanfunc;
mod datelessthanfunc;
mod func;
mod ipaddressfunc;
mod jwt;
mod key;
mod name;
mod nullfunc;
mod numericequalsfunc;
mod numericgreaterfunc;
mod numericlessfunc;
mod stringequalsfunc;
mod stringequalsignorecasefunc;
mod stringlikefunc;
mod value;

pub use binaryequalsfunc::*;
pub use boolfunc::*;
pub use dateequalsfunc::*;
pub use dategreaterthanfunc::*;
pub use datelessthanfunc::*;
pub use func::*;
pub use ipaddressfunc::*;
pub use jwt::*;
pub use key::*;
pub use name::*;
pub use nullfunc::*;
pub use numericequalsfunc::*;
pub use numericgreaterfunc::*;
pub use numericlessfunc::*;
pub use stringequalsfunc::*;
pub use stringequalsignorecasefunc::*;
pub use stringlikefunc::*;
pub use value::*;

pub(self) fn canonical_key(key: &str) -> String {
    // todo: golang http.CanonicalHeaderKey
    key.to_owned()
}
