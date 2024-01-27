use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::*,
};

use tokio_stream::{wrappers::WatchStream, Stream, StreamExt};

use crate::web::session::LeaderboardEntry;

use super::App;

#[derive(Template)]
#[template(path = "contest/leaderboard.html")]
pub struct LeaderboardPage {
    session_id: i64,
    contest_name: String,
}

pub async fn leaderboard(
    State(app): State<App>,
    Path(session_id): Path<i64>,
) -> Result<LeaderboardPage, StatusCode> {
    Ok(LeaderboardPage {
        session_id,
        contest_name: {
            let sessions = app.sessions.read().await;
            let session = &sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;
            session.contest.name.clone()
        },
    })
}

#[derive(Template)]
#[template(path = "contest/leaderboard_rankings.html")]
pub struct LeaderboardRankings {
    rankings: Vec<LeaderboardEntry>,
}

pub async fn leaderboard_rankings(
    State(app): State<App>,
    Path(session_id): Path<i64>,
) -> Result<LeaderboardRankings, StatusCode> {
    let sessions = app.sessions.read().await;
    let session = &sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;
    let leaderboard_size = session.contest.leaderboard_size;

    Ok(LeaderboardRankings {
        rankings: session
            .leaderboard
            .rankings()
            .take(leaderboard_size)
            .collect(),
    })
}

pub async fn leaderboard_sse(
    State(app): State<App>,
    Path(session_id): Path<i64>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::Error>>>, StatusCode> {
    let sessions = app.sessions.read().await;
    let session = &sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Sse::new(
        WatchStream::new(session.rx.clone()).map(|_| Ok(Event::default().event("leaderboard"))),
    )
    .keep_alive(KeepAlive::new()))
}
