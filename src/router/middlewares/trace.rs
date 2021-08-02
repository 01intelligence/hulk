use std::borrow::Cow;
use std::error::Error as StdError;
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
use bstr::ByteSlice;
use futures_core::Stream;
use futures_util::future::Either;
use futures_util::{ready, FutureExt};

use crate::globals::{Get, Guard, GLOBALS};
use crate::http::{self, RequestExtensionsContext};
use crate::{admin, utils};

#[derive(Clone)]
pub struct Trace {
    only_headers: bool,
    collect_stats: Option<Cow<'static, str>>,
}

impl Trace {
    pub fn new() -> Self {
        Trace {
            only_headers: true,
            collect_stats: None,
        }
    }

    pub fn trace_all(mut self) -> Self {
        self.only_headers = false;
        self
    }

    pub fn collect_stats<T: Into<Cow<'static, str>>>(mut self, api: T) -> Self {
        self.collect_stats = Some(api.into());
        self
    }
}

impl Default for Trace {
    fn default() -> Self {
        Self::new()
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
            collect_stats: self.collect_stats.clone(),
        }))
    }
}

pub struct TraceMiddleware<S> {
    service: S,
    only_headers: bool,
    collect_stats: Option<Cow<'static, str>>,
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
        let stats_guard = if let Some(api) = &self.collect_stats {
            Some(
                GLOBALS
                    .http_stats
                    .current_s3_requests
                    .inc_guard(api.clone()),
            )
        } else {
            None
        };

        if GLOBALS.trace.subscribers_num() == 0 {
            return RecordResponse {
                fut: self.service.call(req),
                inner: None,
                rx: None,
                _phantom: PhantomData,
                stats_guard,
            };
        }

        let fn_name = sanitize_operation_name(req.request().ctx().handler_fn_name.unwrap());

        let mut bytes_read = 0;
        for (k, v) in req.headers() {
            bytes_read += format!("{}: {}\n", k, v.to_str().unwrap_or_default()).len();
        }

        let (tx, rx) = tokio::sync::oneshot::channel();

        let (request, payload) = req.into_parts();
        let payload = RecordRequestPayload {
            payload,
            bytes_read,
            record_body: if !self.only_headers {
                Some(BytesMut::new())
            } else {
                None
            },
            tx: Some(tx),
        };
        let req = ServiceRequest::from_parts(request, Payload::Stream(Box::pin(payload)));

        let mut node_name = req.head().uri.host().unwrap_or_default();
        let node_name = if node_name.is_empty() || GLOBALS.is_dist_erasure.get() {
            GLOBALS.local_node_name.guard().clone()
        } else {
            node_name.to_owned()
        };

        let req_info = admin::TraceRequestInfo {
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

        let mut trace_info = admin::TraceInfo {
            trace_type: admin::TraceType::Http,
            node_name,
            fn_name,
            time: utils::now(),
            req_info: Some(req_info),
            ..Default::default()
        };

        RecordResponse {
            fut: self.service.call(req),
            inner: Some(RecordResponseInner {
                record: true,
                record_error_only: true,
                trace_info: Some(trace_info),
            }),
            rx: Some(rx),
            stats_guard,
            _phantom: PhantomData,
        }
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
    inner: Option<RecordResponseInner>,
    #[pin]
    rx: Option<tokio::sync::oneshot::Receiver<(usize, Option<Bytes>)>>,
    stats_guard: Option<http::HttpApiStatsGuard<'static>>,
    _phantom: PhantomData<B>,
}

struct RecordResponseInner {
    record: bool,
    record_error_only: bool,
    trace_info: Option<admin::TraceInfo>,
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

        let inner = match this.inner.as_mut() {
            None => {
                return Poll::Ready(Ok(
                    res.map_body(move |_, body| RecordResponseBody { body, inner: None })
                ))
            }
            Some(inner) => inner,
        };

        let (bytes_read, record_body) = match ready!(this.rx.as_pin_mut().unwrap().poll(cx)) {
            Ok(r) => r,
            Err(err) => {
                let err = Box::new(err) as Box<(dyn StdError + 'static)>;
                return Poll::Ready(Err(err.into()));
            }
        };

        let mut trace_info: admin::TraceInfo = inner.trace_info.take().unwrap();
        let rq = trace_info.req_info.as_mut().unwrap();
        rq.body = Some(record_body.unwrap_or_else(|| Bytes::from_static(b"<BODY>")));
        trace_info.call_stats = Some(admin::TraceCallStats {
            input_bytes: bytes_read,
            ..Default::default()
        });

        let status_code: StatusCode = res.status();
        let mut bytes_written = format!(
            "{} {}\n",
            status_code.as_str(),
            status_code.canonical_reason().unwrap_or_default()
        )
        .len();
        let headers: HeaderMap = res.headers().clone();
        for (k, v) in &headers {
            bytes_written += format!("{}: {}\n", k, v.to_str().unwrap_or_default()).len();
        }

        let record_body = if inner.record {
            Some(BytesMut::new())
        } else {
            None
        };
        let record_error_only = inner.record_error_only;

        let rs = admin::TraceResponseInfo {
            time: utils::now(),
            headers: Some(headers),
            status_code,
            body: None,
        };
        trace_info.resp_info = Some(rs);

        let stats_guard = this.stats_guard.take();

        Poll::Ready(Ok(res.map_body(move |_, body| RecordResponseBody {
            body,
            inner: Some(RecordResponseBodyInner {
                status_code,
                bytes_written,
                time_to_first_byte: Duration::ZERO,
                start_time: utils::now(),
                record_body,
                record_error_only,
                trace_info: Some(trace_info),
                stats_guard,
            }),
        })))
    }
}

#[pin_project::pin_project]
struct RecordRequestPayload<S> {
    #[pin]
    payload: Payload<S>,
    bytes_read: usize,
    record_body: Option<BytesMut>,
    tx: Option<tokio::sync::oneshot::Sender<(usize, Option<Bytes>)>>,
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
            Poll::Ready(None) => {
                this.tx.take().unwrap().send((
                    *this.bytes_read,
                    this.record_body.take().map(|b| b.freeze()),
                ));
                Poll::Ready(None)
            }
            p => p,
        }
    }
}

#[pin_project::pin_project]
pub struct RecordResponseBody<B> {
    #[pin]
    body: B,
    inner: Option<RecordResponseBodyInner>,
}

struct RecordResponseBodyInner {
    status_code: StatusCode,
    bytes_written: usize,
    time_to_first_byte: Duration,
    start_time: utils::DateTime,
    record_body: Option<BytesMut>,
    record_error_only: bool,
    trace_info: Option<admin::TraceInfo>,
    stats_guard: Option<http::HttpApiStatsGuard<'static>>,
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
        let inner = match this.inner.as_mut() {
            None => {
                return match ready!(this.body.poll_next(cx)) {
                    Some(Err(err)) => Poll::Ready(Some(Err(err.into()))),
                    Some(Ok(r)) => Poll::Ready(Some(Ok(r))),
                    None => Poll::Ready(None),
                };
            }
            Some(inner) => inner,
        };

        match ready!(this.body.poll_next(cx)) {
            Some(Ok(chunk)) => {
                inner.bytes_written += chunk.len();
                if inner.time_to_first_byte == Duration::ZERO {
                    inner.time_to_first_byte = utils::now()
                        .signed_duration_since(inner.start_time)
                        .to_std()
                        .unwrap_or_default();
                }
                if let Some(record) = &mut inner.record_body {
                    if !inner.record_error_only || inner.status_code >= StatusCode::BAD_REQUEST {
                        record.put_slice(chunk.as_ref());
                    }
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Some(Err(err)) => Poll::Ready(Some(Err(err.into()))),
            None => {
                let mut trace_info: admin::TraceInfo = inner.trace_info.take().unwrap();

                let req_info = trace_info.req_info.as_mut().unwrap();
                let resp_info = trace_info.resp_info.as_mut().unwrap();
                resp_info.time = utils::now();
                resp_info.body = Some(
                    inner
                        .record_body
                        .take()
                        .map(|b| b.freeze())
                        .unwrap_or_else(|| Bytes::from_static(b"<BODY>")),
                );

                let call_stats = trace_info.call_stats.as_mut().unwrap();
                call_stats.latency = resp_info
                    .time
                    .signed_duration_since(inner.start_time)
                    .to_std()
                    .unwrap_or_default();
                call_stats.output_bytes = inner.bytes_written;
                call_stats.time_to_first_byte = inner.time_to_first_byte;

                let stats_guard: Option<http::HttpApiStatsGuard<'_>> = inner.stats_guard.take();
                if let Some(stats_guard) = stats_guard {
                    GLOBALS.http_stats.update_stats(
                        stats_guard.api(),
                        resp_info.status_code,
                        &req_info.path,
                    );
                }

                GLOBALS.trace.publish(trace_info);

                Poll::Ready(None)
            }
        }
    }
}

fn sanitize_operation_name(name: &str) -> String {
    return name.to_owned();
}
