// SPDX-License-Identifier: Apache-2.0
use std::pin::Pin;
use crate::error::registry::RegistryError;
use crate::registry::repository::Repository;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

/// Interface for reading and storing blobs
#[async_trait]
pub trait RepositoryTrait {
    /// Persists a blob to the underlying storage driver
    async fn persist(&self, repo: Repository) -> Result<Pin<Box<dyn AsyncWrite>>, RegistryError>;

    /// Get a buf reader from the underlying storage driver
    async fn read(&self, repo: Repository) -> Result<Pin<Box<dyn AsyncRead>>, RegistryError>;

}