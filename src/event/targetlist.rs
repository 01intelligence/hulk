use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::ensure;
use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use super::*;

// Event target trait.
#[async_trait]
pub trait Target {
    fn id(&self) -> &TargetId;
    fn is_active(&self) -> anyhow::Result<bool>;
    async fn save(&self, event: &Event) -> anyhow::Result<()>;
    async fn send(&self, s: &str) -> anyhow::Result<()>;
    async fn close(&mut self) -> anyhow::Result<()>;
    fn has_queue_store(&self) -> bool;
}

#[derive(Default)]
pub struct TargetList(HashMap<TargetId, Arc<Mutex<Box<dyn Target>>>>);

impl TargetList {
    pub fn add(&mut self, target: Box<dyn Target>) -> anyhow::Result<()> {
        ensure!(
            self.0.contains_key(target.id()),
            "target {} already exists",
            target.id()
        );
        self.0
            .insert(target.id().clone(), Arc::new(Mutex::new(target)));
        Ok(())
    }

    pub fn contains(&self, id: &TargetId) -> bool {
        self.0.contains_key(id)
    }

    pub async fn remove(&mut self, id: &TargetId) {
        if let Some(mut target) = self.0.remove(id) {
            let _ = target.lock().await.close().await;
        }
    }

    pub fn targets(&self) -> Vec<Arc<Mutex<Box<dyn Target>>>> {
        self.0.values().cloned().collect()
    }

    pub fn iter(
        &self,
    ) -> std::collections::hash_map::Iter<'_, TargetId, Arc<Mutex<Box<dyn Target>>>> {
        self.0.iter()
    }

    pub fn iter_mut(
        &mut self,
    ) -> std::collections::hash_map::IterMut<'_, TargetId, Arc<Mutex<Box<dyn Target>>>> {
        self.0.iter_mut()
    }

    pub async fn send(
        &mut self,
        event: Event,
        targets: HashSet<TargetId>,
        tx: UnboundedSender<(TargetId, Option<anyhow::Error>)>,
    ) {
        let targets: Vec<_> = targets.into_iter().collect();
        let mut results = Vec::new();
        for id in &targets {
            match self.0.get_mut(id) {
                Some(target) => {
                    let target = target.clone();
                    let event = &event;
                    let tx = tx.clone();
                    results.push(async move {
                        let r = target.lock().await.save(event).await;
                        let _ = tx.send((id.clone(), r.err()));
                    });
                }
                None => {
                    let _ = tx.send((id.clone(), None));
                }
            }
        }
        let _ = futures_util::future::join_all(results).await;
    }
}
