use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    sync::Arc,
};

use thiserror::Error;
use time::OffsetDateTime;

use super::{database::Database, Contest};

#[derive(Debug, Clone)]
pub struct Session {
    pub id: i64,

    // Contest
    pub contest: Arc<Contest>,
    pub start: Option<OffsetDateTime>,
    pub end: Option<OffsetDateTime>,

    // Users
    pub leaderboard: BinaryHeap<LeaderboardEntry>,
    pub cooldowns: HashMap<(i64, i64), OffsetDateTime>,
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
            start: None,
            end: None,
            cooldowns: HashMap::new(),
            leaderboard: BinaryHeap::new(),
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

    pub fn update_leaderboard(&mut self, entry: LeaderboardEntry) -> bool {
        let mut new = BinaryHeap::new();
        let mut updated = false;

        while let Some(current) = self.leaderboard.pop() {
            if current.user_id == entry.user_id {
                new.push(LeaderboardEntry {
                    score: current.score.max(entry.score),
                    ..current
                });
                updated = true;
            } else {
                new.push(current);
            }
        }

        if !updated {
            new.push(entry);
        }

        self.leaderboard = new;
        updated
    }
}

#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub score: u32,
    pub username: String,
    pub user_id: i64,
}

impl PartialEq for LeaderboardEntry {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for LeaderboardEntry {}

impl PartialOrd for LeaderboardEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LeaderboardEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}
