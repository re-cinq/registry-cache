// SPDX-License-Identifier: Apache-2.0
use std::path::PathBuf;
use std::pin::Pin;
use async_trait::async_trait;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncWrite};
use crate::driver::RepositoryTrait;
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;
use crate::registry::repository::Repository;

#[derive(Clone)]
pub struct FilesystemStorage {
    app_config: crate::config::app::AppConfig
}

#[async_trait]
impl RepositoryTrait for FilesystemStorage {

    async fn persist(&self, repo: Repository) -> Result<Pin<Box<dyn AsyncWrite>>, RegistryError> {

        // Get the blob path
        let blob_path = self.blob_path(repo);

        // Open the blob file
        let blob_file = self.open_file_for_write(&blob_path).await.map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

        // Box it and pin it
        Ok(Box::pin(blob_file))

    }

    async fn read(&self, repo: Repository) -> Result<Pin<Box<dyn AsyncRead>>, RegistryError> {
        // Get the blob path
        let blob_path = self.blob_path(repo);

        // Open the blob file
        let blob_file = self.open_file_for_read(&blob_path).await.map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

        // Box it and pin it
        Ok(Box::pin(blob_file))
    }
}

impl FilesystemStorage {

    /// New instance of the FilesystemStorage
    pub fn new(app_config: crate::config::app::AppConfig) -> FilesystemStorage {
        FilesystemStorage {
            app_config
        }
    }

    /// Build the local blob path
    pub fn blob_path(&self, repo: Repository) -> PathBuf {
        // Extract the digest
        let digest = repo.digest.unwrap();

        // Build the path where to store the data
        PathBuf::from(self.app_config.storage.folder.to_string()).join(digest.algo.to_string()).join(digest.hash)

    }

    pub fn blob_path_tmp(&self, repo: Repository) -> PathBuf {
        // Extract the digest
        let digest = repo.digest.unwrap();

        // Build the path where to store the data
        PathBuf::from(self.app_config.storage.folder.to_string()).join(digest.algo.to_string()).join(format!("{}_tmp", digest.hash))

    }

    /// Get an async read File handle
    async fn open_file_for_read(&self, file_path: &PathBuf) -> Result<File,  std::io::Error> {
        // Create the file options
        let mut options = OpenOptions::new();

        // We need to have a reference otherwise the Options get freed
        let options = options.read(true);

        // Now open the file
        options.open(&file_path).await

    }

    /// Get an async read/write/create File handle
    async fn open_file_for_write(&self, file_path: &PathBuf) -> Result<File,  std::io::Error> {
        // Create the file options
        let mut options = OpenOptions::new();

        // We need to have a reference otherwise the Options get freed
        let options = options.read(true).write(true).create(true);

        // Now open the file
        options.open(&file_path).await

    }

}