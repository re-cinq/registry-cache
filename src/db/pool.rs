// SPDX-License-Identifier: Apache-2.0
use sqlx::{Executor, SqlitePool};
use sqlx::sqlite::SqlitePoolOptions;
use crate::config::db::DBConfig;
use crate::db::db_manifests::DBManifests;

/// Database Pool
pub struct DBPool;

impl DBPool {

    /// Create a new DB Pool from the DBConfig parameter
    pub async fn from_config(config: &DBConfig) -> SqlitePool {
        // Build the pool from the config file
        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(config.max_connections)
            .connect(&config.uri)
            .await.expect("Failed to create Database pool");

        pool.execute("PRAGMA journal_mode=WAL;");
        pool.execute("PRAGMA cache_size=10000;");

        // Create the table
        DBManifests::create_table(&pool).await;

        return pool;
    }

    pub async fn default() -> SqlitePool {
        SqlitePoolOptions::new()
            .min_connections(5)
            .max_connections(10)
            .connect("sqlite::memory:")
            .await.expect("Failed to create Database pool")
    }
}