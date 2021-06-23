mod action;
pub mod condition;
mod effect;
mod id;
mod policy;
mod principal;
mod resource;
mod statement;

pub use action::*;
pub use effect::*;
pub use id::*;
pub use policy::*;
pub use principal::*;
pub use resource::*;
pub use statement::*;

pub trait Valid {
    // Checks if self is valid or not.
    fn is_valid(&self) -> bool;
}

pub trait Allowed {
    // Returns if given check is allowed or not.
    fn is_allowed(&self, b: bool) -> bool;
}

pub trait AsSlice<T> {
    fn as_slice(&self) -> &[T];
}

impl<T> AsSlice<T> for T {
    fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self as *const T, 1) }
    }
}

impl<T> AsSlice<T> for [T] {
    fn as_slice(&self) -> &[T] {
        self
    }
}

pub trait ToVec<T> {
    fn to_vec(&self) -> Vec<T>;
}
