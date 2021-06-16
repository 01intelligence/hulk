mod id;
mod effect;

pub use id::*;
pub use effect::*;

pub trait Valid {
    // Checks if self is valid or not.
    fn is_valid(&self) -> bool;
}

pub trait Allowed {
    // Returns if given check is allowed or not.
    fn is_allowed(&self, b: bool) -> bool;
}
