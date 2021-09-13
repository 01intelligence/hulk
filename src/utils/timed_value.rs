use std::future::Future;
use std::pin::Pin;

use tokio::sync::RwLock;

use super::{Duration, Instant};

pub type TimedValueUpdateFnResult<T> =
    Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + Sync>>;

pub type TimedValueUpdateFn<T> = Box<dyn Fn() -> TimedValueUpdateFnResult<T> + Send + Sync>;

pub struct TimedValue<T> {
    update: Option<TimedValueUpdateFn<T>>,
    ttl: Duration,
    inner: RwLock<Inner<T>>,
}

struct Inner<T> {
    last_update: Instant,
    value: Option<T>,
}

unsafe impl<T: Clone> Sync for TimedValue<T> {}

impl<T: Clone> TimedValue<T> {
    pub fn new(ttl: Option<Duration>, update: Option<TimedValueUpdateFn<T>>) -> Self {
        Self {
            update,
            ttl: ttl.unwrap_or_else(|| Duration::from_secs(1)),
            inner: RwLock::new(Inner {
                last_update: Instant::now(),
                value: None,
            }),
        }
    }

    pub async fn get<Fut, F>(&self, update: Option<F>) -> anyhow::Result<T>
    where
        Fut: Future<Output = anyhow::Result<T>>,
        F: FnOnce() -> Fut,
    {
        let inner = self.inner.read().await;
        if inner.last_update.elapsed() < self.ttl {
            if let Some(value) = &inner.value {
                return Ok(value.clone());
            }
        }
        drop(inner);

        let value = if let Some(update) = update {
            update().await?
        } else {
            (self.update.as_ref().unwrap())().await?
        };
        let mut inner = self.inner.write().await;
        inner.value = Some(value);
        inner.last_update = Instant::now();
        Ok(inner.value.as_ref().unwrap().clone())
    }
}
