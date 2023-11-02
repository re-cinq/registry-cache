// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::Arc;
use crate::config::app::{AppConfig, UpstreamConfig};
use crate::handlers::command::blob::service::ManifestService;
use crate::pubsub::command_bus::CommandBus;
use crate::repository::filesystem::FilesystemStorage;

#[derive(Clone)]
pub struct AppState {
    pub client: reqwest::Client,
    pub command_bus: Arc<CommandBus>,
    pub app_config: AppConfig,
    pub storage: FilesystemStorage,
    pub upstreams: HashMap<String, UpstreamConfig>,
    pub manifests: Arc<ManifestService>
}

impl AppState {
    pub fn new(client: reqwest::Client, command_bus: Arc<CommandBus>, app_config: AppConfig, storage: FilesystemStorage, manifests: Arc<ManifestService>) -> Self {
        AppState {
            client,
            command_bus,
            upstreams: app_config.upstreams(),
            app_config,
            storage,
            manifests
        }
    }
}