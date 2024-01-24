use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::web::session::LeaderboardEntry;

use super::App;

#[derive(Template)]
#[template(path = "contest/leaderboard.html")]
pub struct Leaderboard {
    session_id: i64,
    contest_name: String,
    rankings: Vec<LeaderboardEntry>,
}

pub async fn leaderboard(
    State(app): State<App>,
    Path(session_id): Path<i64>,
) -> Result<Leaderboard, StatusCode> {
    let sessions = app.sessions.read().await;
    let session = &sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Leaderboard {
        session_id,
        contest_name: session.contest.name.clone(),
        rankings: session
            .leaderboard
            .rankings()
            .take(session.contest.leaderboard_size)
            .collect(),
    })
}
