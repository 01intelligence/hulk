use std::cell::{Ref, RefMut};
use std::collections::HashMap;

use actix_web::http::HeaderMap;
use actix_web::HttpRequest;

pub struct RequestExtensions {
    pub handler_fn_name: Option<&'static str>,
    pub special_headers: Option<HeaderMap>,
    pub extra: Option<HashMap<String, String>>,
}

pub trait RequestExtensionsContext {
    fn ctx(&self) -> Ref<'_, RequestExtensions>;
    fn ctx_mut(&self) -> RefMut<'_, RequestExtensions>;
}

impl RequestExtensionsContext for HttpRequest {
    fn ctx(&self) -> Ref<'_, RequestExtensions> {
        Ref::map(self.extensions(), |e| e.get::<RequestExtensions>().unwrap())
    }

    fn ctx_mut(&self) -> RefMut<'_, RequestExtensions> {
        RefMut::map(self.extensions_mut(), |e| {
            e.get_mut::<RequestExtensions>().unwrap()
        })
    }
}
