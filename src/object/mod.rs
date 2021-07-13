mod api_datatypes;
mod api_errors;
mod api_trait;
mod api_utils;

pub use api_datatypes::*;
pub use api_errors::*;
pub use api_trait::*;
pub use api_utils::*;

pub fn new_object_layer() -> anyhow::Result<Box<dyn ObjectLayer>> {
    todo!()
}
