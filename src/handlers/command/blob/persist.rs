// SPDX-License-Identifier: Apache-2.0
use std::sync::Arc;
use async_trait::async_trait;
use bytes::Bytes;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::mpsc::UnboundedReceiver;
use crate::handlers::command::blob::service::ManifestService;
use crate::models::commands::RegistryCommand;
use crate::models::events::RegistryEvent;
use crate::pubsub::subscriber::CommandSubscriberTrait;
use crate::registry::digest::Digest;
use crate::registry::repository::Repository;
use crate::repository::filesystem::FilesystemStorage;

/// Manages the blob persistence
pub struct BlobPersistHandler {
    service: Arc<FilesystemStorage>,
    manifests: Arc<ManifestService>
}

impl BlobPersistHandler {

    /// Create a new ARC wrapped instance of the RoleAddSubscriber
    pub fn new(service: Arc<FilesystemStorage>, manifests: Arc<ManifestService>) -> Arc<Self> {
        Arc::new(BlobPersistHandler {
            service,
            manifests
        })
    }

    /// Persists the blob and verifies its sha256
    async fn persist(&self, repository: Repository, mut receiver: UnboundedReceiver<Bytes>) -> Option<RegistryEvent> {
        // The original digest
        let original_digest = repository.clone().digest.unwrap();

        // Build the blob file path
        let file_path_tmp = self.service.blob_path_tmp(repository.clone());
        let file_path_final = self.service.blob_path(repository.clone());

        // Create the file options
        let mut options = OpenOptions::new();

        // We need to have a reference otherwise the Options get freed
        let options = options.read(true).write(true).create(true);

        // Now open the file
        let file = options.open(&file_path_tmp).await;

        // Check if we could open a file handle
        match file {
            // Success
            Ok(mut file) => {

                // Process the chunks coming from upstream and store them in the tmp file
                while let Some(chunk) = receiver.recv().await {
                    // Write the whole chunk
                    if let Err(e) = file.write(chunk.as_ref()).await {
                        tracing::error!("Failed to persist blob: {}", e.to_string());
                        return None;
                    }
                }

                // Sync all the data to disk, so that we can calculate the file hash
                if let Err(e) = file.sync_data().await {
                    tracing::error!("Failed to sync file to disk: {} {}", original_digest, e.to_string());
                    return None;
                }

                if let Err(e) = file.rewind().await {
                    tracing::error!("Failed to rewind file {} {}", original_digest, e.to_string());
                    return None;
                }

                // Calculate the sha256 to make sure the cached content is valid
                let std_file = file.into_std().await;
                let blob_digest = Digest::hash_digest_file(original_digest.algo, std_file).await;

                match blob_digest {
                    Ok(blob_digest) => {
                        // This means that the digest are different, so there corrupted data
                        if blob_digest != original_digest {

                            // log it
                            tracing::error!("Digest mismatch {} - {}", blob_digest, original_digest);

                            // delete the file now - no reason to keep around broken data
                            if let Err(e) = tokio::fs::remove_file(file_path_tmp).await {
                                tracing::error!("Failed to remove corrupted blob: {}", e.to_string());
                            }
                            return None;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to calculate blob digest: {}", e.to_string());
                        return None;
                    }
                }

                // if we got here, it means the blob was stored successfully and the digest was good

                // Now move the file from a tmp one to the final one
                if let Err(e) = tokio::fs::rename(file_path_tmp, file_path_final).await {
                    tracing::error!("Failed to rename blob: {}", e.to_string());
                    return None;
                }


                tracing::info!("Blob stored in cache successfully: {}/{}", repository.name, original_digest);
            }
            Err(e) => {
                tracing::error!("failed to persist blob: {:?} {}", file_path_final, e.to_string());
                return None
            }
        }

        Some(RegistryEvent::BlobPersisted)
    }
}

#[async_trait]
impl CommandSubscriberTrait for BlobPersistHandler {
    async fn run(&self, cmd: RegistryCommand) -> Option<RegistryEvent> {

        match cmd {
            RegistryCommand::Shutdown => {
                None
            }
            RegistryCommand::PersistBlob(repository, receiver) => {
                self.persist(repository, receiver).await
            }
            RegistryCommand::PersistManifest(repository, digest, size, mime, receiver) => {

                match digest {
                    Some(digest) => {
                        // Build the repository with the sha256 of the manifest
                        let manifest_digest = digest.to_string();

                        // Build the manifest repository with the sha256 of the manifest
                        let manifest_repository = Repository::new_with_reference(&repository.name, &manifest_digest);

                        // Make sure we build the manifest correctly
                        match manifest_repository {
                            Ok(manifest_repository) => {

                                // File system persistence
                                if let Some(RegistryEvent::BlobPersisted) = self.persist(manifest_repository, receiver).await {

                                    // Database index persistence
                                    if let Err(e) = self.manifests.persist(&repository, digest, size, &mime).await {
                                        tracing::error!("failed to persist manifest index: {}", e.to_string());
                                        return None;
                                    }

                                    return Some(RegistryEvent::BlobPersisted);
                                }
                                None
                            }
                            Err(e) => {
                                tracing::error!("failed to build manifest repository: {}", e.to_string());
                                None
                            }
                        }
                    }
                    None => None
                }
            }
        }


    }

    fn supports_concurrency(&self) -> bool {
        true
    }
}