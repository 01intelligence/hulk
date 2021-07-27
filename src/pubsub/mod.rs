use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;

#[derive(Clone)]
pub struct PubSub<T: Clone> {
    tx: broadcast::Sender<T>,
    subscribers: Arc<AtomicUsize>,
}

pub struct Receiver<T: Clone>(broadcast::Receiver<T>, Arc<AtomicUsize>);

impl<T: Clone> Receiver<T> {
    pub async fn recv(&mut self) -> anyhow::Result<T> {
        loop {
            match self.0.recv().await {
                // Ignore Lagged error and continue next recv.
                Err(err) => {
                    // TODO: Wait for exported broadcast::RecvError::Lagged.
                    if !err.to_string().contains("lagged") {
                        return Err(err.into());
                    }
                }
                // Err(broadcast::RecvError::Lagged(_)) => {}
                Ok(r) => {
                    return Ok(r);
                }
            }
        }
    }
}

impl<T: Clone> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.1.fetch_sub(1, Ordering::Relaxed);
    }
}

impl<T: Clone> PubSub<T> {
    pub fn new(capacity: usize) -> PubSub<T> {
        let (tx, _) = broadcast::channel(capacity);
        PubSub {
            tx,
            subscribers: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn publish(&self, value: T) {
        let _ = self.tx.send(value); // ignore error
    }

    pub fn subscribe(&self) -> Receiver<T> {
        self.subscribers.fetch_add(1, Ordering::Relaxed);
        Receiver(self.tx.subscribe(), self.subscribers.clone())
    }

    pub fn subscribers_num(&self) -> usize {
        self.subscribers.load(Ordering::Relaxed)
    }
}
