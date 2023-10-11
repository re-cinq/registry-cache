// SPDX-License-Identifier: Apache-2.0
use std::sync::Arc;
use async_trait::async_trait;
use crate::models::commands::RegistryCommand;
use crate::models::events::RegistryEvent;

/// Command Pub Sub Bus Trait
#[async_trait]
pub trait CommandSubscriberTrait {
    /// The function to execute when a message of interest is received
    async fn run(&self, cmd: RegistryCommand) -> Option<RegistryEvent>;

    /// Whether the run operation can be executed concurrently
    fn supports_concurrency(&self) -> bool;
}

/// Event Pub Sub Bus Trait
#[async_trait]
pub trait EventSubscriberTrait {
    /// The function to execute when a message of interest is received
    async fn run(&self, event: &RegistryEvent) -> Option<RegistryEvent>;

    /// Receive the responses from the run operation
    fn responder(&self) -> Option<tokio::sync::mpsc::Sender<RegistryEvent>>;

    /// Whether the run operation can be executed concurrently
    fn supports_concurrency(&self) -> bool;
}

pub type CommandSubscriber = Arc<dyn CommandSubscriberTrait + 'static + Sync + Send>;
// pub type EventSubscriber = Arc<dyn EventSubscriberTrait + 'static + Sync + Send>;