// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use config::{Config, File};
use serde::{Deserialize, Serialize};
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;

const CONFIG_FILE_NAME:&str = "config.yaml";

/// Configuration for the cache itself
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub upstreams: Vec<UpstreamConfig>,
    pub storage: StorageConfig
}

impl From<Config> for AppConfig {
    fn from(c: Config) -> Self {
        c.try_deserialize().unwrap()
    }
}

impl AppConfig {

    /// Load a specific Application Config
    pub fn load_file(source: &str) -> Result<AppConfig, RegistryError> {
        let config = Config::builder()
            .add_source(File::with_name(source))
            .build().unwrap();
        config.try_into().map_err(|_e| RegistryError::new(ErrorKind::ConfigError)
            .with_error(format!("Failed to read config file {}", source)))
    }

    /// Load the default config file: config.yaml
    pub fn load() -> Result<AppConfig, RegistryError> {
        AppConfig::load_file(CONFIG_FILE_NAME)
    }

    /// Whether the AppConfig is valid
    pub fn is_valid(&self) -> bool {

        // We need the hostname both for the realm and the oidc redirections
        if self.api.hostname.is_empty() {
            tracing::error!("config.yaml has an empty api->hostname");
            return false;
        }

        true
    }

    pub fn upstreams(&self) -> HashMap<String, UpstreamConfig> {
        let mut config = HashMap::default();
        for upstream in &self.upstreams {
            config.insert(upstream.host.clone(), upstream.clone());
        }
        config
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageConfig {
    pub folder: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpstreamConfig {
    pub host: String,
    pub registry: String,
    pub port: u16,
    pub schema: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiConfig {

    /// Hostname this is the exposed hostname of the registry
    pub hostname: String,

    /// The address to listen to
    pub address: Option<String>,

    /// The port to listen to
    pub port: Option<String>,

    /// The ipv6 address to listen to
    pub address_ipv6: Option<String>,

    /// The ipv6 port to listen to
    pub port_ipv6: Option<String>,

    /// The location of the TLS key file
    pub tls_key: Option<String>,

    /// The location of the TLS cert file
    pub tls_cert: Option<String>
}
