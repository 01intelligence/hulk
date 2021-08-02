use std::cell::{Ref, RefMut};
use std::collections::HashMap;

use actix_web::http::HeaderMap;
use actix_web::web::Query;
use actix_web::HttpRequest;

pub struct RequestExtensions {
    // Handler function name.
    pub handler_fn_name: Option<&'static str>,
    // Special headers which are added to response but useful for some middlewares.
    pub special_headers: Option<HeaderMap>,
    // Parsed query map.
    pub query: Option<Option<Query<HashMap<String, String>>>>,
    // Extra metadata.
    pub extra: Option<HashMap<String, String>>,
}

pub trait RequestExtensionsContext {
    fn ctx(&self) -> Ref<'_, RequestExtensions>;
    fn ctx_mut(&self) -> RefMut<'_, RequestExtensions>;
    fn query(&self) -> Ref<'_, Option<Query<HashMap<String, String>>>>;
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

    fn query(&self) -> Ref<'_, Option<Query<HashMap<String, String>>>> {
        let mut e = self.ctx_mut();
        let _ = e.query.get_or_insert_with(|| {
            // Not parsed yet, so parse it.
            Query::<HashMap<String, String>>::from_query(self.query_string())
                .map(|q| Some(q))
                // Parse failed, but insert None.
                .unwrap_or(None)
        });
        Ref::map(self.ctx(), |e| e.query.as_ref().unwrap())
    }
}
