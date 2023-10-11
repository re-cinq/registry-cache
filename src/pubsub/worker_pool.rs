// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::RwLock;
use tracing::log;
use crate::models::commands::RegistryCommand;
use crate::pubsub::command::ChannelId;


/// CommandWorkerPool
/// Dispatches the commands to sub workers
pub struct WorkerPool {

    /// Sender to queue events
    queue: Sender<RegistryCommand>,

    /// Subscribers is a map of events, as keys and
    /// as values, a list of functions to execute when that specific event is processed
    subscribers: Arc<RwLock<HashMap<u64, Sender<RegistryCommand>>>>,

    /// The modulo we want to calculate
    modulo: u64
}

/// CommandWorkerPool
impl WorkerPool {

    /// New instance
    pub fn new(queue: Sender<RegistryCommand>) -> Arc<WorkerPool> {
        Arc::new(WorkerPool {
            queue,
            subscribers: Arc::new(Default::default()),
            modulo: num_cpus::get() as u64
        })
    }

    /// Start processing the events
    pub async fn start(&self, mut receiver: Receiver<RegistryCommand>) {
        // Wait to get a command
        while let Some(cmd) = receiver.recv().await {

            // Get the current subscribers
            let guard = self.subscribers.read().await;

            // Command queue id
            let queue_id = cmd.queue_id();

            // Get the channel ID we should use
            let channel_id = queue_id % self.modulo;

            // Get the list of subscribers for the specific channel id
            let subscriber = guard.get(&channel_id);

            // If we have some
            if let Some(subscriber) = subscriber {

                // create a local reference that can be passed to a different thread
                // The subscriber is wrapped around an Arc so all that clone does is increment a
                // counter for the memory reference
                let local_subscriber = subscriber.clone();

                // Do the work in a different async task
                tokio::spawn(async move {

                    log::debug!("Queued command {} on channel {}", cmd.topic_id(), channel_id);

                    // Queue the master command for processing
                    if let Err(e) = local_subscriber.send(cmd).await {
                        log::error!("failed to send result back to subscriber: {:?}", e.to_string());
                    }
                });
            } else {
                log::error!("WARNING: subscriber not found!")
            }
        }
    }

    /// Publish asynchronously a new event in the bus
    pub async fn publish(&self, cmd: RegistryCommand) {
        if let Err(e) = self.queue.send(cmd).await {
            log::error!("failed to queue event with error: {:?}", e.to_string());
        }
    }

    /// Subscribe a subscriber to a topic
    pub async fn subscribe(&self, worker_id: usize, subscriber: Sender<RegistryCommand>) {
        let mut writer = self.subscribers.write().await;
        writer.insert(worker_id as u64, subscriber);
    }

    pub async fn shutdown(&self, ) {
        let subs =  self.subscribers.write().await;
        for (index, sub) in subs.iter() {
            tracing::info!("Shutting down worker pool: {}", index);
            if (sub.send(RegistryCommand::Shutdown).await).is_err() {
                continue;
            } else {
                sub.closed().await;
            }
        }
    }
}