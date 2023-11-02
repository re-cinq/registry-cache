use sqlx::{Row, Error, Executor, SqlitePool};
use sqlx::sqlite::SqliteRow;
use crate::models::manifest_record::ManifestRecord;
use crate::registry::digest::Digest;

/// Return the sha256 of the manifest for the specific container image name and tag
const MANIFEST_FOR_TAG:&str = "SELECT name, tag, reference, size, mime FROM manifests where name = $1 AND tag = $2;";

/// Upsert a record in the manifests table
const MANIFEST_UPSERT_QUERY: &str = "INSERT INTO manifests (name, tag, reference, size, mime) VALUES ($1, $2, $3, $4, $5) ON CONFLICT(name, tag) DO UPDATE SET reference=EXCLUDED.reference;";

/// Delete a manifest
const MANIFEST_DELETE_QUERY: &str = "DELETE FROM manifests WHERE name = $1 AND tag = $2;";

/// DANGER: Delete all records
const MANIFEST_DELETE_ALL:&str = "DELETE from manifests;";

/// Create the manifests database table
const MANIFESTS_TABLE:&str = r#"
-- CREATORS
CREATE TABLE IF NOT EXISTS manifests (
name             TEXT NOT NULL,
tag              TEXT NOT NULL,
reference        TEXT NOT NULL,
size             INTEGER NOT NULL,
mime             TEXT NOT NULL,
PRIMARY KEY(name, tag)
);

CREATE INDEX IF NOT EXISTS manifests_name_ids ON manifests(name);
CREATE INDEX IF NOT EXISTS manifests_tag_ids ON manifests(tag);
CREATE INDEX IF NOT EXISTS manifests_reference_ids ON manifests(reference);

"#;

/// Database Manifests Helper
pub struct DBManifests;

impl DBManifests {

    /// Parse the database row
    fn parse(row: SqliteRow) -> ManifestRecord {
        let parsed_digest = Digest::parse(row.get(2)).ok();
        ManifestRecord::new(row.get(0), row.get(1),
                            parsed_digest, row.get(3),
                            row.get(4))
    }

    /// Creates the database table
    pub async fn create_table(pool: &SqlitePool) {
        pool.execute(MANIFESTS_TABLE).await.expect("Failed to create the 'manifests' table");
    }

    /// Return an optional manifest record
    pub async fn manifest_for_tag(pool: &SqlitePool, name: &str, tag: &str) -> Result<Option<ManifestRecord>, Error> {

        sqlx::query(MANIFEST_FOR_TAG)
            .bind(name)
            .bind(tag)
            .map(|row: SqliteRow| {
                DBManifests::parse(row)
            })
            .fetch_optional(pool).await

    }

    /// Deletes an entry in the manifest table
    pub async fn delete(pool: &SqlitePool, name: &str, tag: &str) -> Result<u64, Error> {

        // Build the query
        let query = sqlx::query(MANIFEST_DELETE_QUERY)
            .bind(name)
            .bind(tag)
            .execute(pool);

        // Execute it
        Ok(query.await?.rows_affected())
    }

    /// Upsert a manifest
    pub async fn upsert(pool: &SqlitePool, name: &str, tag: &str, reference: Digest, size: i32, mime: &str) -> Result<u64, Error> {

        let digest = reference.to_string();

        let query = sqlx::query(MANIFEST_UPSERT_QUERY)
            .bind(name)
            .bind(tag)
            .bind(digest)
            .bind(size)
            .bind(mime);

        Ok(query.execute(pool).await?.rows_affected())
    }

    /// Delete all matches (used for testing purposes only)
    #[allow(dead_code)]
    pub async fn delete_all(pool: &SqlitePool) -> Result<u64, Error> {

        let total = sqlx::query(MANIFEST_DELETE_ALL).execute(pool)
            .await?.rows_affected();

        Ok(total)

    }
}

#[cfg(test)]
mod test {
    use crate::db::db_manifests::DBManifests;
    use crate::db::pool::DBPool;
    use crate::registry::digest::Digest;

    #[tokio::test]
    async fn db_manifests_test() {

        // Get an in memory database
        let pool = DBPool::default().await;

        let name = String::from("nvidia/cuda");
        let tag = String::from("12.2.0-devel-ubuntu20.04");
        let digest = Digest::parse("sha256:c1d07892979445e720a5cf1f5abe6a910f45c6d638bf9997d6a807924eee5190").expect("Failed to parse digest");
        let updated_digest = Digest::parse("sha256:77c8fe4188129f39831d01bd626696d8bbff5831180eb8061041181e1b1d17a0").expect("Failed to parse updated digest");
        let mime = "application/vnd.docker.distribution.manifest.v2+json";
        let size = 5117;


        // Create the database table
        DBManifests::create_table(&pool).await;
        DBManifests::delete_all(&pool).await.expect("Failed to truncate manifests table");

        // add a a new record
        let total = DBManifests::upsert(&pool, &name, &tag, digest.clone(), size, mime).await.expect("Failed to upsert manifest record");
        assert_eq!(1, total);

        // get the manifest for the name and tag
        let manifest = DBManifests::manifest_for_tag(&pool, &name, &tag).await.expect("Failed to get manifest for image");

        // Assert we got a manifest
        assert!(manifest.is_some());

        // Unwrap it and make sure it was parsed correctly
        let manifest = manifest.unwrap();
        assert_eq!(name, manifest.name);
        assert_eq!(tag, manifest.tag);
        assert_eq!(digest, manifest.reference.unwrap());
        assert_eq!(size, manifest.size);
        assert_eq!(mime, manifest.mime);

        // Try the upsert functionality now
        let total = DBManifests::upsert( &pool, &name, &tag, updated_digest.clone(), size, mime).await.expect("Failed to update manifest");
        assert_eq!(1, total);

        // check if manifest for an image exists
        let manifest = DBManifests::manifest_for_tag(&pool, &name, &tag).await.expect("Failed to get manifest for image");
        assert!(manifest.is_some());

        let manifest = manifest.unwrap();
        assert_eq!(name, manifest.name);
        assert_eq!(tag, manifest.tag);
        assert_eq!(updated_digest, manifest.reference.unwrap());

        // Delete the record
        let total = DBManifests::delete(&pool, &name, &tag).await.expect("Failed to delete manifest record");
        assert_eq!(1, total);
    }
}