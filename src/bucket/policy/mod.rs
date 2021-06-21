pub mod condition;
mod effect;
mod id;
mod principal;
mod resource;
mod statement;

pub use effect::*;
pub use id::*;
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

pub trait ToSlice<T> {
    fn to_slice(&self) -> &[T];
}

impl<T> ToSlice<T> for T {
    fn to_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self as *const T, 1) }
    }
}

impl<T> ToSlice<T> for [T] {
    fn to_slice(&self) -> &[T] {
        self
    }
}

pub trait ToVec<T> {
    fn to_vec(&self) -> Vec<T>;
}
