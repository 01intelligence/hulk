mod api_interface;
mod api_utils;

pub use api_interface::*;
pub use api_utils::*;

pub fn new_object_layer() -> anyhow::Result<Box<dyn ObjectLayer>> {
    todo!()
}
