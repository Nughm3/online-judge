use std::{collections::HashMap, sync::Arc};

use thiserror::Error;
use time::OffsetDateTime;

use super::{database::Database, Contest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: i64,
    pub contest: Arc<Contest>,
    pub user_cooldowns: HashMap<(i64, i64), OffsetDateTime>,
    pub start: Option<OffsetDateTime>,
    pub end: Option<OffsetDateTime>,
}

pub type SessionResult<T> = Result<T, SessionError>;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("failed to update database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    InvalidAction(&'static str),
}

impl Session {
    pub async fn new(db: &Database, contest: Arc<Contest>) -> SessionResult<Self> {
        let contest_name = contest.name.clone();
        let contest_path = contest.path.display().to_string();

        let id = sqlx::query!(
            "INSERT INTO sessions (contest_name, contest_path) VALUES (?, ?);",
            contest_name,
            contest_path,
        )
        .execute(db.pool())
        .await?
        .last_insert_rowid();

        Ok(Session {
            id,
            contest,
            user_cooldowns: HashMap::new(),
            start: None,
            end: None,
        })
    }

    pub async fn start(&mut self, db: &Database) -> SessionResult<()> {
        if self.start.is_none() && self.end.is_none() {
            let now = OffsetDateTime::now_utc();

            sqlx::query!("UPDATE sessions SET start = ? WHERE id = ?;", now, self.id)
                .execute(db.pool())
                .await?;
            self.start = Some(now);

            Ok(())
        } else {
            Err(SessionError::InvalidAction(
                "tried to start session that has already started",
            ))
        }
    }

    pub async fn end(&mut self, db: &Database) -> SessionResult<()> {
        if self.start.is_none() {
            Err(SessionError::InvalidAction(
                "tried to end session that hasn't started",
            ))
        } else if self.end.is_some() {
            Err(SessionError::InvalidAction(
                "tried to end session that has already ended",
            ))
        } else {
            let now = OffsetDateTime::now_utc();

            sqlx::query!("UPDATE sessions SET end = ? WHERE id = ?;", now, self.id)
                .execute(db.pool())
                .await?;

            self.end = Some(now);

            Ok(())
        }
    }
}
