use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use super::App;
use crate::web::{error::*, session::LeaderboardEntry};

#[derive(Template)]
#[template(path = "contest/leaderboard.html")]
pub struct Leaderboard {
    session_id: i64,
    contest_name: String,
    rankings: Vec<LeaderboardEntry>,
}

pub async fn leaderboard(
    State(app): State<Arc<App>>,
    Path(session_id): Path<i64>,
) -> AppResult<Leaderboard> {
    app.clone()
        .sessions
        .read()
        .await
        .get(&session_id)
        .map(|session| {
            let contest = &session.contest;
            Leaderboard {
                session_id,
                contest_name: contest.name.clone(),
                rankings: session
                    .leaderboard
                    .rankings()
                    .take(contest.leaderboard_size)
                    .collect(),
            }
        })
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))
}
