use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use lazy_static::lazy_static;
use tonic::transport::{Channel, Endpoint, Error};

use crate::globals::{Guard, GLOBALS};
use crate::utils;

lazy_static! {
    static ref GLOBAL_INTER_NODE_CLIENT_BUILDER: Arc<Mutex<Option<InterNodeClientBuilder>>> =
        Arc::new(Mutex::new(None));
}

#[derive(Clone)]
pub struct InterNodeClientBuilder {
    tls_config: tonic::transport::ClientTlsConfig,
}

impl InterNodeClientBuilder {
    pub fn build(self, uri: &str) -> anyhow::Result<HealthCheckChannel> {
        let uri = tonic::transport::Uri::from_str(uri)?;

        // TODO: proxy, dns cache, ...

        let channel = Endpoint::from(uri)
            .connect_timeout(utils::seconds(15))
            .timeout(utils::minutes(15))
            .tcp_keepalive(Some(utils::seconds(15)))
            .tls_config(self.tls_config)?
            .connect_lazy()?;

        let channel = tower::ServiceBuilder::new()
            .layer_fn(HealthCheckChannel::new)
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
    let active_cred = GLOBALS.active_cred.guard();
    crate::http::authenticate_node(active_cred.access_key.clone(), &active_cred.secret_key).unwrap()
}

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::task::{Context, Poll};

use http::{HeaderValue, Request, Response};
use tonic::body::BoxBody;
use tonic::transport::Body;
use tonic::Code;
use tower::Service;

use crate::errors::AsError;
use crate::utils::DateTimeFormatExt;

pub struct HealthCheckChannel {
    inner: Channel,
    health_check_fn: Option<Box<dyn Fn() -> bool>>,
    connected: AtomicBool,
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
    last_connected: AtomicI64,
    health_check_interval: utils::Duration,
}

unsafe impl Send for HealthCheckChannel {}

impl HealthCheckChannel {
    fn new(inner: Channel) -> Self {
        Self {
            inner,
            health_check_fn: None,
            connected: AtomicBool::new(false),
            health_check_handle: None,
            last_connected: AtomicI64::new(utils::now().timestamp_nanos()),
            health_check_interval: Default::default(),
        }
    }

    fn is_online(&self) -> bool {
        todo!()
    }

    fn mark_offline(&mut self) {
        if self.health_check_fn.is_some() {
            if let Ok(_) =
                self.connected
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            {
                let this = utils::SendRawPtr::new(self as *mut Self);
                let handle = tokio::spawn(async move {
                    // Safety: `this` should never outlive `self`.
                    let this = unsafe { this.to().as_mut().unwrap() };

                    let mut rng = utils::rng_seed_now();
                    loop {
                        if (this.health_check_fn.as_ref().unwrap())() {
                            if let Ok(_) = this.connected.compare_exchange(
                                false,
                                true,
                                Ordering::SeqCst,
                                Ordering::SeqCst,
                            ) {
                                // TODO: log
                                this.last_connected
                                    .store(utils::now().timestamp_nanos(), Ordering::Relaxed);
                                return;
                            }
                        }
                        utils::sleep(this.health_check_interval, Some(&mut rng)).await;
                    }
                });
                self.health_check_handle = Some(handle);
            }
        }
    }
}

impl Service<Request<BoxBody>> for HealthCheckChannel {
    type Response = Response<Body>;
    type Error = anyhow::Error;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<BoxBody>) -> Self::Future {
        if !self.is_online() {}

        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        let inner_cloned = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner_cloned);

        let this = utils::SendRawPtr::new(self as *mut Self);
        Box::pin(async move {
            // Safety: `this` should never outlive `self`.
            let this = unsafe { this.to().as_mut().unwrap() };

            let headers = req.headers_mut();
            let _ = headers.insert(
                "Authorization",
                HeaderValue::try_from(format!("Bearer {}", new_auth_token()))?,
            );
            let _ = headers.insert(
                "X-Hulk-Time",
                HeaderValue::try_from(utils::now().rfc3339())?,
            );

            match inner.call(req).await {
                Ok(rep) => Ok(rep),
                Err(err) => {
                    if let Some(status) = err.as_error::<tonic::Status>() {
                        match status.code() {
                            Code::Unavailable | Code::FailedPrecondition => {
                                this.mark_offline();
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
