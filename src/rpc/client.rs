use std::convert::TryFrom;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use http::{HeaderValue, Request, Response};
use lazy_static::lazy_static;
use tonic::body::BoxBody;
use tonic::transport::{Body, Channel, Endpoint};
use tonic::Code;
use tower::Service;

use crate::errors::AsError;
use crate::globals::{Guard, ReadWriteGuard, GLOBALS};
use crate::utils;
use crate::utils::{DateTimeExt, DateTimeFormatExt};

lazy_static! {
    static ref GLOBAL_INTER_NODE_CLIENT_BUILDER: Arc<Mutex<Option<InterNodeClientBuilder>>> =
        Arc::new(Mutex::new(None));
}

#[derive(Clone)]
pub struct InterNodeClientBuilder {
    tls_config: tonic::transport::ClientTlsConfig,
}

impl InterNodeClientBuilder {
    pub fn build(self, endpoint: &crate::endpoint::Endpoint) -> anyhow::Result<WrappedChannel> {
        let uri = tonic::transport::Uri::from_str(endpoint.url())?;

        // TODO: proxy, dns cache, ...

        let channel = Endpoint::from(uri)
            .connect_timeout(utils::seconds(15))
            .timeout(utils::minutes(15))
            .tcp_keepalive(Some(utils::seconds(15)))
            .tls_config(self.tls_config)?
            .connect_lazy()?;

        let channel = tower::ServiceBuilder::new()
            .layer_fn(WrappedChannel::new)
            .service(channel);

        Ok(channel)
    }
}

pub fn get_inter_node_client_builder() -> InterNodeClientBuilder {
    // Safety: builder must exist.
    GLOBAL_INTER_NODE_CLIENT_BUILDER
        .lock()
        .unwrap()
        .clone()
        .unwrap()
}

pub fn set_inter_node_client_builder(ca_certs: Option<&[u8]>) -> anyhow::Result<()> {
    let mut tls_config = tonic::transport::ClientTlsConfig::new();
    if let Some(ca_certs) = ca_certs {
        tls_config = tls_config.ca_certificate(tonic::transport::Certificate::from_pem(ca_certs));
    }

    let builder = InterNodeClientBuilder { tls_config };

    *GLOBAL_INTER_NODE_CLIENT_BUILDER.lock().unwrap() = Some(builder);

    Ok(())
}

pub fn new_auth_token() -> String {
    let active_cred = GLOBALS.active_cred.read_guard();
    crate::http::authenticate_node(active_cred.access_key.clone(), &active_cred.secret_key).unwrap()
}

pub struct WrappedChannel {
    channel: Channel,
    inner: Arc<WrappedChannelInner>,
    tx: Arc<tokio::sync::Notify>,
}

struct WrappedChannelInner {
    health_check_fn: Mutex<Option<HealthCheckFn>>,
    health_check_interval: utils::Duration,
    connected: AtomicBool,
    last_connected: AtomicI64,
    rx: Arc<tokio::sync::Notify>,
}

type HealthCheckFnFuture = Pin<Box<dyn Future<Output = bool> + Send>>;
type HealthCheckFn = Box<dyn Fn() -> HealthCheckFnFuture + Send + Sync>;

unsafe impl Send for WrappedChannelInner {}
unsafe impl Sync for WrappedChannelInner {}

impl Drop for WrappedChannel {
    fn drop(&mut self) {
        self.tx.notify_one();
    }
}

impl WrappedChannel {
    fn new(inner: Channel) -> Self {
        let notify = Arc::new(tokio::sync::Notify::new());

        Self {
            channel: inner,
            inner: Arc::new(WrappedChannelInner {
                health_check_fn: Mutex::new(None),
                health_check_interval: utils::milliseconds(200),
                connected: AtomicBool::new(false),
                last_connected: AtomicI64::new(utils::now().timestamp_nanos()),
                rx: notify.clone(),
            }),
            tx: notify,
        }
    }

    pub fn health_check_setter(&self) -> Box<dyn FnOnce(HealthCheckFn)> {
        let inner = self.inner.clone();
        Box::new(move |health_check: HealthCheckFn| {
            *inner.health_check_fn.lock().unwrap() = Some(health_check);
        })
    }

    pub fn is_online(&self) -> bool {
        self.inner.connected.load(Ordering::Relaxed)
    }

    pub fn last_connected(&self) -> utils::DateTime {
        utils::DateTime::from_timestamp_nanos(self.inner.last_connected.load(Ordering::Relaxed))
    }

    fn mark_offline(inner: Arc<WrappedChannelInner>) {
        let mut health_check_guard = match inner.health_check_fn.try_lock() {
            Ok(h) => h,
            Err(_) => return,
        };

        if let Err(_) =
            inner
                .connected
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        {
            return;
        }

        let health_check = health_check_guard.take().unwrap();
        drop(health_check_guard);
        let _ = tokio::spawn(async move {
            let mut rng = utils::rng_seed_now();
            loop {
                if health_check().await {
                    if let Ok(_) = inner.connected.compare_exchange(
                        false,
                        true,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        // TODO: log
                        inner
                            .last_connected
                            .store(utils::now().timestamp_nanos(), Ordering::Relaxed);
                        break;
                    }
                }

                tokio::select! {
                    _ = utils::sleep(inner.health_check_interval, Some(&mut rng)) => {},
                    _ = inner.rx.notified() => {
                        break;
                    },
                }
            }
            let _ = inner.health_check_fn.lock().unwrap().insert(health_check);
        });
    }
}

impl Service<Request<BoxBody>> for WrappedChannel {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.channel.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<BoxBody>) -> Self::Future {
        if !self.is_online() {
            return Box::pin(async { Err(anyhow::anyhow!("remote node offline")) });
        }

        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        let channel_cloned = self.channel.clone();
        let mut channel = std::mem::replace(&mut self.channel, channel_cloned);

        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let headers = req.headers_mut();
            let _ = headers.insert(
                "Authorization",
                HeaderValue::try_from(format!("Bearer {}", new_auth_token()))?,
            );
            let _ = headers.insert(
                "X-Hulk-Time",
                HeaderValue::try_from(utils::now().rfc3339())?,
            );

            match channel.call(req).await {
                Ok(rep) => Ok(rep),
                Err(err) => {
                    if let Some(status) = err.as_error::<tonic::Status>() {
                        match status.code() {
                            Code::Unavailable | Code::FailedPrecondition => {
                                Self::mark_offline(inner);
                            }
                            _ => {}
                        }
                    }
                    Err(err.into())
                }
            }
        })
    }
}
