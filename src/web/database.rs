use std::str::FromStr;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("failed to apply database migrations: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

impl Database {
    pub async fn new(url: &str) -> Result<Self, DatabaseError> {
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .optimize_on_close(true, 400);

        let pool = SqlitePool::connect_with(options).await?;
        sqlx::migrate!().run(&pool).await?;

        Ok(Database { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
