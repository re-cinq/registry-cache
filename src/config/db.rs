use serde::{Deserialize, Serialize};

// SPDX-License-Identifier: Apache-2.0
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DBConfig {
    pub max_connections: u32,
    pub uri: String
}

impl Default for DBConfig {
    fn default() -> Self {
        DBConfig {
            max_connections: 1,
            // uri: "sqlite:/tmp/cache/cache.db".to_string()
            uri: "sqlite::memory:".to_string()
        }
    }
}