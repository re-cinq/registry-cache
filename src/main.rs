// SPDX-License-Identifier: Apache-2.0
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::config::app::AppConfig;
use crate::handlers::command::filesystem::persist::BlobPersistHandler;
use crate::models::commands::PERSIST_BLOB;
use crate::pubsub::command_bus::CommandBus;
use crate::repository::filesystem::FilesystemStorage;

mod api;
mod error;
mod registry;
mod config;
mod repository;
mod driver;
mod pubsub;
mod models;
mod handlers;
mod metrics;
mod db;

#[tokio::main]
async fn main() -> std::io::Result<()> {

    // Logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "cache_registry=info,tower_http=debug,axum::rejection=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get access to the config
    let config = AppConfig::load().expect("Application Config error");
    if !config.is_valid() {
        return Ok(tracing::error!("invalid config.yaml"));
    }

    // Init the command bus
    let queue_size = 4096;
    let (command_sender, command_receiver) = tokio::sync::mpsc::channel(queue_size);
    let command_bus = CommandBus::new(command_sender, queue_size);
    let local_command_bus = command_bus.clone();
    tokio::spawn(async move {
        local_command_bus.start(command_receiver).await;
    });

    // Subscribe the persistence handler
    let storage = Arc::new(FilesystemStorage::new(config.clone()));
    command_bus.subscribe(PERSIST_BLOB.to_string(), BlobPersistHandler::new(storage)).await;

    // Start the API server
    if let Err(e) = api::server::start(config.clone(), command_bus.clone()).await {
        tracing::info!("Error shutting down registry cache {}", e);
    }

    tracing::info!("Shutdown completed");

    Ok(())

}
