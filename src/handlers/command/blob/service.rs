use std::sync::Arc;
use sqlx::SqlitePool;
use crate::config::db::DBConfig;
use crate::db::db_manifests::DBManifests;
use crate::db::pool::DBPool;
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;
use crate::models::manifest_record::ManifestRecord;
use crate::models::types::MimeType;
use crate::registry::digest::Digest;
use crate::registry::repository::Repository;

pub struct ManifestService {
    pool: SqlitePool
}

impl ManifestService {
    pub async fn new(db_config: &DBConfig) -> Arc<ManifestService> {
        Arc::new(ManifestService {
            pool: DBPool::from_config(db_config).await,
        })
    }

    /// Persists a link between an image tag and a digest
    pub async fn persist(&self, repository: &Repository, reference: Digest, size: i32, mime: &MimeType) -> Result<u64, RegistryError> {
        DBManifests::upsert(&self.pool, &repository.components.join("/"), &repository.reference, reference, size, mime).await
            .map_err(|e| RegistryError::new(ErrorKind::RegistryManifestInvalid).with_error(e.to_string()))
    }

    /// Get a reference from a tag name
    pub async fn get(&self, repository: &Repository) -> Result<Option<ManifestRecord>, RegistryError> {
        DBManifests::manifest_for_tag(&self.pool, &repository.components.join("/"), &repository.reference).await
            .map_err(|e| RegistryError::new(ErrorKind::RegistryManifestInvalid).with_error(e.to_string()))
    }
}