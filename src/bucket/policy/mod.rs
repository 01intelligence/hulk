mod id;
mod effect;
mod resource;
pub mod condition;

pub use id::*;
pub use effect::*;
pub use resource::*;

pub trait Valid {
    // Checks if self is valid or not.
    fn is_valid(&self) -> bool;
}

pub trait Allowed {
    // Returns if given check is allowed or not.
    fn is_allowed(&self, b: bool) -> bool;
}
