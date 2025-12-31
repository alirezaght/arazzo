use sqlx::PgPool;

use crate::store::StoreError;

pub async fn run_migrations(pool: &PgPool) -> Result<(), StoreError> {
    let migrator = sqlx::migrate!("postgres/migrations");
    let result: Result<(), sqlx::migrate::MigrateError> = migrator.run(pool).await;
    result.map_err(|e| StoreError::Other(e.to_string()))?;
    Ok(())
}

