use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use tokio_stream::StreamExt;

use super::{App, Pagination};
use crate::web::error::*;

#[derive(Template)]
#[template(path = "contest/leaderboard.html")]
pub struct Leaderboard {
    contest_name: String,
    session_id: i64,
}

pub async fn leaderboard(
    State(app): State<Arc<App>>,
    Path(session_id): Path<i64>,
) -> AppResult<Leaderboard> {
    app
        .clone()
        .sessions
        .read()
        .await
        .get(&session_id)
        .map(|session| Leaderboard {
            session_id,
            contest_name: session.contest.name.clone(),
        })
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))
}

#[derive(Template)]
#[template(path = "contest/leaderboard_table.html")]
pub struct LeaderboardTable {
    session_id: i64,
    page: usize,
    rankings: Vec<(String, i64, u32)>,
    more: bool,
}

pub async fn leaderboard_table(
    State(app): State<Arc<App>>,
    Path(session_id): Path<i64>,
    Query(Pagination { page }): Query<Pagination>,
) -> AppResult<LeaderboardTable> {
    let offset = 10 * (page - 1) as i64;

    let rankings: Vec<_> = sqlx::query!(
        "SELECT users.username, users.id, SUM(max_score) AS total_score
         FROM (
           SELECT submissions.user_id, submissions.task, MAX(submissions.score) AS max_score
           FROM submissions
           WHERE submissions.session_id = ?
           GROUP BY submissions.user_id, submissions.task
         ) subquery
         JOIN users ON subquery.user_id = users.id
         GROUP BY users.username
         ORDER BY total_score DESC
         LIMIT 10 OFFSET ?;",
        session_id,
        offset
    )
    .fetch(app.db.pool())
    .map(|res| res.map(|rank| (rank.username, rank.id, rank.total_score as u32)))
    .collect::<Result<_, _>>()
    .await?;

    let count = sqlx::query!("SELECT COUNT(DISTINCT user_id) AS count FROM submissions;")
        .fetch_one(app.db.pool())
        .await?
        .count as usize;

    Ok(LeaderboardTable {
        session_id,
        page,
        rankings,
        more: count > page * 10,
    })
}
