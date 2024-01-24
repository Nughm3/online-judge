use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use super::{App, ContestNavigation};
use crate::{contest::*, web::auth::AuthSession};

#[derive(Template)]
#[template(path = "contest/contest.html")]
pub struct ContestPage {
    session_id: i64,
    contest: Arc<Contest>,
    started: bool,
    logged_in: bool,
}

pub async fn contest(
    auth_session: AuthSession,
    State(app): State<App>,
    Path(session_id): Path<i64>,
) -> Result<ContestPage, StatusCode> {
    let sessions = app.sessions.read().await;
    let session = sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(ContestPage {
        session_id,
        contest: session.contest.clone(),
        started: session.start.is_some(),
        logged_in: auth_session.user.is_some(),
    })
}

#[derive(Template)]
#[template(path = "contest/task.html")]
pub struct TaskPage {
    session_id: i64,
    contest_name: String,
    task_id: i64,
    has_prev: bool,
    has_next: bool,
    task: Task,
}

pub async fn task(
    State(app): State<App>,
    Path(ContestNavigation {
        session_id,
        task_id,
    }): Path<ContestNavigation>,
) -> Result<TaskPage, StatusCode> {
    let sessions = app.sessions.read().await;
    let session = sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;

    let task = session
        .contest
        .tasks
        .get(task_id as usize - 1)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(TaskPage {
        session_id,
        contest_name: session.contest.name.clone(),
        task_id,
        has_prev: task_id > 1,
        has_next: task_id < session.contest.tasks.len() as i64,
        task,
    })
}
