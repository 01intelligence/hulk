use actix_web::dev::AnyBody;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse};

// Standard cross domain policy information located at https://s3.amazonaws.com/crossdomain.xml
const CROSS_DOMAIN_XML: &str = r#"<?xml version="1.0"?><!DOCTYPE cross-domain-policy SYSTEM "http://www.adobe.com/xml/dtds/cross-domain-policy.dtd"><cross-domain-policy><allow-access-from domain="*" secure="false" /></cross-domain-policy>"#;

// Standard path where an app would find cross domain policy information.
const CROSS_DOMAIN_XMLENTITY: &str = "/crossdomain.xml";

// A cross-domain policy file is an XML document that grants a web client, such as Adobe Flash Player
// or Adobe Acrobat (though not necessarily limited to these), permission to handle data across domains.
// When clients request content hosted on a particular source domain and that content make requests
// directed towards a domain other than its own, the remote domain needs to host a cross-domain
// policy file that grants access to the source domain, allowing the client to continue the transaction.
pub fn cross_domain_policy(req: &HttpRequest) -> Option<HttpResponse> {
    if req.path() == CROSS_DOMAIN_XMLENTITY {
        return Some(HttpResponse::with_body(
            StatusCode::OK,
            AnyBody::from(CROSS_DOMAIN_XML),
        ));
    }
    None
}
