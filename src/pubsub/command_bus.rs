// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;
use tracing::log;
use crate::models::commands::RegistryCommand;
use crate::pubsub::subscriber::{CommandSubscriber};
use crate::pubsub::worker::Worker;
use crate::pubsub::worker_pool::WorkerPool;

/// Command Bus
/// Dispatches the commands to the subscribers
pub struct CommandBus {

    /// Sender to queue events
    queue: tokio::sync::mpsc::Sender<RegistryCommand>,

    /// Subscribers is a map of events, as keys and
    /// as values, a list of functions to execute when that specific event is processed
    subscribers: Arc<RwLock<HashMap<String, Arc<WorkerPool>>>>,

    /// Amount of CPUs the server has
    cpus: usize,

    /// The size of the workers channel
    buffer_size: usize,

    /// Whether the bus is shutting down
    shutting_down: AtomicBool
}

/// Bus
impl CommandBus {

    /// New instance
    pub fn new(queue: tokio::sync::mpsc::Sender<RegistryCommand>, buffer_size: usize) -> Arc<CommandBus> {

        Arc::new(CommandBus {
            queue,
            subscribers: Arc::new(Default::default()),
            cpus: num_cpus::get(),
            buffer_size,
            shutting_down: Default::default(),
        })
    }

    pub async fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
        for (topic, pool) in self.subscribers.write().await.iter() {
            tracing::info!("Shutting down worker pool for topic: {}", topic);
            pool.shutdown().await;
        }
    }

    /// Start processing the events
    pub async fn start(&self, mut receiver: tokio::sync::mpsc::Receiver<RegistryCommand>) {
        while let Some(exec) = receiver.recv().await {

            let guard = self.subscribers.read().await;

            // Get thew list if subscribers for the specific command
            let worker_pool = guard.get(&exec.topic());

            // If we have some
            if let Some(worker_pool) = worker_pool {
                worker_pool.publish(exec).await;
            }
        }
    }

    /// Publish asynchronously a new event in the bus
    pub async fn publish(&self, exec: RegistryCommand) {

        // If we are already shutting down, do not queue any messages
        if self.shutting_down.load(Ordering::Relaxed) {
            log::warn!("Command bus is shutting down - command not delivered");
            return;
        }

        if let Err(e) = self.queue.send(exec).await {
            log::error!("failed to queue event with error: {:?}", e);
        }
    }

    /// Subscribe a subscriber to a topic
    pub async fn subscribe(&self, topic: String, handler: CommandSubscriber) {

        // Mutable subscribers
        let mut subscribers = self.subscribers.write().await;

        // If we don't have a worker pool for this kind of topic
        // then add it
        if subscribers.get(&topic).is_none() {
            // Create the channel
            let (event_sender, event_receiver) = tokio::sync::mpsc::channel(4096);

            // Create the pool
            let worker_pool = WorkerPool::new(event_sender);

            // Clone it
            let worker_pool_clone = worker_pool.clone();

            // Start listening for messages
            tokio::spawn(async move {
                worker_pool_clone.start(event_receiver).await
            });

            // Now create the N amount of channels
            // Persist the data to the disk for each entity
            for channel in 0..self.cpus {

                // Start a parallel sink
                let worker = Worker::new(self.buffer_size, handler.clone());

                // Start the processing in background
                let sender = worker.start().await;

                // Subscribe the sink to the worker pool
                worker_pool.subscribe(channel, sender).await;
            }

            // Add the pool
            subscribers.insert(topic, worker_pool);

        }
    }
}