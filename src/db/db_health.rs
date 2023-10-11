// SPDX-License-Identifier: Apache-2.0
use sqlx::SqlitePool;

// Query for checking the DB connection
const HEALTH:&str = "SELECT 1;";

pub struct DBHealth {}

impl DBHealth {

    /// Check the DB connection
    pub async fn health(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(HEALTH).fetch_all(pool).await?;
        Ok(())
    }

}

#[cfg(test)]
mod test {
    use crate::db::pool::DBPool;

    #[tokio::test]
    async fn db_health_test() {

        let pool = DBPool::default().await;
        let result = super::DBHealth::health(&pool).await;
        assert!(result.is_ok());

    }
}