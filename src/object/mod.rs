mod api_interface;

pub use api_interface::*;

pub fn new_object_layer() -> anyhow::Result<Box<dyn ObjectLayer>> {
    todo!()
}
