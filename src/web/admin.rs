use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use axum_login::{permission_required, AuthzBackend};
use serde::Deserialize;
use tokio_stream::StreamExt;

use super::{
    app::{App, Pagination},
    auth::{AuthSession, Backend, Permissions, User},
    error::*,
    session::Session,
};
use crate::contest::Contest;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/admin", get(admin))
        .route("/admin/sessions", get(session_table).post(sessions_action))
        .route(
            "/admin/contests",
            get(contest_table).post(contest_create_session),
        )
        .route("/admin/users", get(user_table).post(user_action))
        .route_layer(permission_required!(Backend, Permissions::ADMIN))
        .with_state(app)
}

#[derive(Template)]
#[template(path = "admin/admin.html")]
struct AdminPage;

async fn admin() -> AdminPage {
    AdminPage
}

#[derive(Template)]
#[template(path = "admin/session_table.html")]
struct SessionTable {
    page: usize,
    sessions: Vec<Session>,
    more: bool,
}

async fn session_table(
    State(app): State<Arc<App>>,
    Query(Pagination { page }): Query<Pagination>,
) -> SessionTable {
    let sessions = &app.sessions.lock().await;

    SessionTable {
        page,
        sessions: sessions
            .values()
            .skip((page - 1) * 10)
            .take(10)
            .cloned()
            .collect(),
        more: sessions.len() > page * 10,
    }
}

#[derive(Debug, Deserialize)]
struct SessionQuery {
    id: i64,
    action: SessionAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SessionAction {
    Start,
    End,
}

#[derive(Template)]
#[template(path = "admin/session_control.html")]
struct SessionControl {
    id: i64,
    started: bool,
    ended: bool,
}

async fn sessions_action(
    State(app): State<Arc<App>>,
    Query(query): Query<SessionQuery>,
) -> AppResult<SessionControl> {
    let app = app.clone();
    let sessions = &mut app.sessions.lock().await;
    let session = sessions
        .get_mut(&query.id)
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

    match query.action {
        SessionAction::Start => {
            {
                let app = app.clone();
                let duration = session.contest.duration;
                tokio::task::spawn(async move {
                    tokio::time::sleep(duration.try_into().expect("invalid contest duration"))
                        .await;

                    let sessions = &mut app.sessions.lock().await;
                    let session = sessions.get_mut(&query.id).unwrap();

                    session.end(&app.db).await.ok();
                });
            }

            session.start(&app.db).await?
        }
        SessionAction::End => session.end(&app.db).await?,
    }

    Ok(SessionControl {
        id: query.id,
        started: session.start.is_some(),
        ended: session.end.is_some(),
    })
}

#[derive(Template)]
#[template(path = "admin/contest_table.html")]
struct ContestTable {
    page: usize,
    contests: Vec<Contest>,
    more: bool,
}

async fn contest_table(
    State(app): State<Arc<App>>,
    Query(Pagination { page }): Query<Pagination>,
) -> ContestTable {
    ContestTable {
        page,
        contests: app
            .contests
            .iter()
            .skip((page - 1) * 10)
            .take(10)
            .cloned()
            .collect(),
        more: app.contests.len() > page * 10,
    }
}

#[derive(Debug, Deserialize)]
struct ContestCreateSession {
    idx: usize,
}

async fn contest_create_session(
    State(app): State<Arc<App>>,
    Query(ContestCreateSession { idx }): Query<ContestCreateSession>,
) -> AppResult<Response> {
    let contest = app.contests[idx - 1].clone();
    let session = Session::new(&app.db, contest).await?;
    app.sessions.lock().await.insert(session.id, session);
    Ok(Response::builder()
        .header("HX-Trigger", "reloadSessions")
        .body("Contest session created".into())?)
}

#[derive(Template)]
#[template(path = "admin/user_table.html")]
struct UserTable {
    page: usize,
    users: Vec<User>,
    more: bool,
}

async fn user_table(
    State(app): State<Arc<App>>,
    Query(Pagination { page }): Query<Pagination>,
) -> AppResult<UserTable> {
    let offset = 10 * (page - 1) as i64;

    let users = sqlx::query!("SELECT * FROM users LIMIT 10 OFFSET ?;", offset)
        .fetch(app.db.pool())
        .map(|res| res.map(|user| User::new(user.id, &user.username, &user.password)))
        .collect::<Result<_, _>>()
        .await?;

    let count = sqlx::query!("SELECT COUNT(*) AS count FROM users;")
        .fetch_one(app.db.pool())
        .await?
        .count as usize;

    Ok(UserTable {
        page,
        users,
        more: count > page * 10,
    })
}

#[derive(Debug, Deserialize)]
struct UserQuery {
    id: i64,
    action: UserAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum UserAction {
    Delete,
}

async fn user_action(
    auth_session: AuthSession,
    State(app): State<Arc<App>>,
    Query(query): Query<UserQuery>,
) -> AppResult<StatusCode> {
    let user = sqlx::query!("SELECT * FROM users WHERE id = ?;", query.id)
        .fetch_optional(app.db.pool())
        .await?
        .map(|user| User::new(user.id, &user.username, &user.password))
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

    Ok(match query.action {
        UserAction::Delete => {
            if auth_session
                .backend
                .has_perm(&user, Permissions::ADMIN)
                .await?
            {
                StatusCode::UNAUTHORIZED
            } else {
                sqlx::query!("DELETE FROM users WHERE id = ?;", query.id)
                    .execute(app.db.pool())
                    .await?;
                StatusCode::OK
            }
        }
    })
}
