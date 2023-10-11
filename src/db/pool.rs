// SPDX-License-Identifier: Apache-2.0
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use crate::config::db::DBConfig;

/// Database Pool
pub struct DBPool;

impl DBPool {

    /// Create a new DB Pool from the DBConfig parameter
    pub async fn from_config(config: &DBConfig) -> SqlitePool {
        // Build the pool from the config file
        SqlitePoolOptions::new()
            .min_connections(5)
            .max_connections(config.max_connections)
            .connect(&config.uri)
            .await.expect("Failed to create Database pool")
    }

    pub async fn default() -> SqlitePool {
        SqlitePoolOptions::new()
            .min_connections(5)
            .max_connections(10)
            .connect("sqlite::memory:")
            .await.expect("Failed to create Database pool")
    }
}