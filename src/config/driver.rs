// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

/// Storage driver for the cache content
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, EnumString, Default)]
pub enum StorageDriver {
    /// Default filesystem
    #[default]
    FileSystem,

    /// Not supported for now
    Distributed
}