use std::future::{ready, Future, Ready};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use actix_http::body::{BodySize, MessageBody};
use actix_http::error::PayloadError;
use actix_http::HttpMessage;
use actix_web::dev::{
    forward_ready, Payload, PayloadStream, Service, ServiceRequest, ServiceResponse, Transform,
};
use actix_web::error::Error;
use actix_web::http::{header, HeaderMap, HeaderName, StatusCode};
use actix_web::web::{BufMut, Bytes, BytesMut};
use futures_core::Stream;
use futures_util::future::{Either, LocalBoxFuture};
use futures_util::{ready, FutureExt};

use crate::globals::{Get, Guard, GLOBALS};
use crate::http::RequestExtensionsContext;
use crate::{admin, utils};

pub struct Trace {
    only_headers: bool,
}

impl Trace {
    pub fn trace_all() -> Self {
        Trace {
            only_headers: false,
        }
    }

    pub fn trace_only_headers() -> Self {
        Trace { only_headers: true }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Trace
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<RecordResponseBody<B>>;
    type Error = Error;
    type Transform = TraceMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TraceMiddleware {
            service,
            only_headers: self.only_headers,
        }))
    }
}

pub struct TraceMiddleware<S> {
    service: S,
    only_headers: bool,
}

impl<S, B> Service<ServiceRequest> for TraceMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Response = ServiceResponse<RecordResponseBody<B>>;
    type Error = Error;
    type Future = RecordResponse<S, B>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let fn_name = sanitize_operation_name(req.request().ctx().handler_fn_name.unwrap());
        let mut headers = req.headers().clone();

        let (request, payload) = req.into_parts();
        let payload = RecordRequestPayload {
            payload,
            bytes_read: 0,
            record_body: if !self.only_headers {
                Some(BytesMut::new())
            } else {
                None
            },
        };
        let req = ServiceRequest::from_parts(request, Payload::Stream(Box::pin(payload)));

        let mut node_name = req.head().uri.host().unwrap_or_default();
        let node_name = if node_name.is_empty() || GLOBALS.is_dist_erasure.get() {
            GLOBALS.local_node_name.guard().clone()
        } else {
            node_name.to_owned()
        };

        let mut t = admin::TraceInfo {
            trace_type: admin::TraceType::Http,
            fn_name,
            time: utils::now(),
            node_name,
            ..Default::default()
        };

        let rq = admin::TraceRequestInfo {
            time: utils::now(),
            proto: format!("{:?}", req.head().version),
            method: req.method().to_string(),
            raw_query: req.query_string().to_owned(),
            client: req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or_default()
                .to_owned(),
            headers: Some(req.headers().clone()),
            path: req.path().to_owned(),
            body: None,
        };

        let res = RecordResponse{
            fut: self.service.call(req),
            record: true,
            record_error_only: true,
            _phantom: PhantomData,
        };

        /*let rs = admin::TraceResponseInfo{
            time: utils::now(),
            headers:,
            status_code:,
            body: None,
        }

        t.req_info = Some(rq);
        t.resp_info = Some(rs);

        ()*/

        res
    }
}

#[pin_project::pin_project]
pub struct RecordResponse<S, B>
where
    S: Service<ServiceRequest>,
    B: MessageBody,
{
    #[pin]
    fut: S::Future,
    record: bool,
    record_error_only: bool,
    _phantom: PhantomData<B>,
}

impl<S, B> Future for RecordResponse<S, B>
where
    B: MessageBody,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Output = Result<ServiceResponse<RecordResponseBody<B>>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let res = match ready!(this.fut.poll(cx)) {
            Ok(res) => res,
            Err(e) => return Poll::Ready(Err(e)),
        };

        let status: StatusCode = res.status();
        let mut bytes_written = format!(
            "{} {}\n",
            status.as_str(),
            status.canonical_reason().unwrap_or_default()
        )
        .len();
        let headers: &HeaderMap = res.headers();
        for (k, v) in headers {
            bytes_written += format!("{}: {}\n", k, v.to_str().unwrap_or_default()).len();
        }

        let record_body = if *this.record {
            Some(BytesMut::new())
        } else {
            None
        };
        let record_error_only = *this.record_error_only;

        Poll::Ready(Ok(res.map_body(move |_, body| RecordResponseBody {
            status,
            body,
            bytes_written,
            time_to_first_byte: Duration::ZERO,
            start_time: utils::now(),
            record_body,
            record_error_only,
        })))
    }
}

#[pin_project::pin_project]
struct RecordRequestPayload<S> {
    #[pin]
    payload: Payload<S>,
    bytes_read: usize,
    record_body: Option<BytesMut>,
}

#[pin_project::pin_project]
pub struct RecordResponseBody<B> {
    status: StatusCode,
    #[pin]
    body: B,
    bytes_written: usize,
    time_to_first_byte: Duration,
    start_time: utils::DateTime,
    record_body: Option<BytesMut>,
    record_error_only: bool,
}

impl<S> Stream for RecordRequestPayload<S>
where
    S: Stream<Item = Result<Bytes, PayloadError>> + Unpin,
{
    type Item = Result<Bytes, PayloadError>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.payload.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                *this.bytes_read += bytes.len();
                if let Some(record) = this.record_body {
                    record.put_slice(bytes.as_ref());
                }
                Poll::Ready(Some(Ok(bytes)))
            }
            p => p,
        }
    }
}

impl<B> MessageBody for RecordResponseBody<B>
where
    B: MessageBody,
    B::Error: Into<Error>,
{
    type Error = Error;

    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Bytes, Self::Error>>> {
        let this = self.project();

        match ready!(this.body.poll_next(cx)) {
            Some(Ok(chunk)) => {
                *this.bytes_written += chunk.len();
                if *this.time_to_first_byte == Duration::ZERO {
                    *this.time_to_first_byte = utils::now()
                        .signed_duration_since(*this.start_time)
                        .to_std()
                        .unwrap_or_default();
                }
                if let Some(record) = this.record_body {
                    if !*this.record_error_only || *this.status >= StatusCode::BAD_REQUEST {
                        record.put_slice(chunk.as_ref());
                    }
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Some(Err(err)) => Poll::Ready(Some(Err(err.into()))),
            None => Poll::Ready(None),
        }
    }
}

fn sanitize_operation_name(name: &str) -> String {
    todo!()
}
