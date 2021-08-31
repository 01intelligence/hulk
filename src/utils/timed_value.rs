use std::future::Future;
use std::pin::Pin;
use std::sync::RwLock;

use futures_util::FutureExt;

use super::{Duration, Instant};

pub trait TimedValueGetter<T> {
    fn get(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<T>> + '_>>;
}

impl<T: Clone, R: Future<Output = anyhow::Result<T>> + Send + Sync, F: Fn() -> R>
    TimedValueGetter<T> for TimedValue<T, R, F>
{
    fn get(&self) -> Pin<Box<dyn Future<Output = anyhow::Result<T>> + '_>> {
        Box::pin(self.get())
    }
}

pub struct TimedValue<T, R: Future<Output = anyhow::Result<T>> + Send + Sync, F: Fn() -> R> {
    update: F,
    ttl: Duration,
    inner: RwLock<Inner<T>>,
}

struct Inner<T> {
    last_update: Instant,
    value: Option<T>,
}

impl<T: Clone, R: Future<Output = anyhow::Result<T>> + Send + Sync, F: Fn() -> R>
    TimedValue<T, R, F>
{
    pub fn new(ttl: Option<Duration>, update: F) -> Self {
        Self {
            update,
            ttl: ttl.unwrap_or_else(|| Duration::from_secs(1)),
            inner: RwLock::new(Inner {
                last_update: Instant::now(),
                value: None,
            }),
        }
    }

    pub async fn get(&self) -> anyhow::Result<T> {
        let inner = self.inner.read().unwrap();
        if inner.last_update.elapsed() < self.ttl {
            if let Some(value) = &inner.value {
                return Ok(value.clone());
            }
        }
        drop(inner);

        let value = (self.update)().boxed_local().await?;
        let mut inner = self.inner.write().unwrap();
        inner.value = Some(value);
        inner.last_update = Instant::now();
        Ok(inner.value.as_ref().unwrap().clone())
    }
}
