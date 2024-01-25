use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use axum_login::{permission_required, AuthzBackend};
use serde::Deserialize;
use tokio::sync::watch::Sender;
use tokio_stream::StreamExt;

use crate::contest::Contest;
use crate::web::{
    app::App,
    auth::{AuthSession, Backend, Permissions, User},
    error::*,
    session::Session,
};

pub fn router(app: App, tx: Arc<Sender<Vec<Arc<Session>>>>) -> Router {
    Router::new()
        .route("/admin", get(move || async { AdminPage }))
        .route("/admin/sessions", get(sessions).post(sessions_action))
        .route("/admin/contests", get(contests).put(create_session))
        .route("/admin/users", get(users).delete(delete_user))
        .route_layer(permission_required!(Backend, Permissions::ADMIN))
        .layer(Extension(tx))
        .with_state(app)
}

#[derive(Debug, Deserialize)]
struct Pagination {
    page: usize,
}

#[derive(Template)]
#[template(path = "admin/admin.html")]
struct AdminPage;

#[derive(Template)]
#[template(path = "admin/session_table.html")]
struct SessionTable {
    page: usize,
    sessions: Vec<Arc<Session>>,
    more: bool,
}

async fn sessions(
    State(app): State<App>,
    Query(Pagination { page }): Query<Pagination>,
) -> SessionTable {
    let sessions = &app.sessions.read().await;

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
    State(app): State<App>,
    Extension(tx): Extension<Arc<Sender<Vec<Arc<Session>>>>>,
    Query(query): Query<SessionQuery>,
) -> AppResult<SessionControl> {
    {
        let sessions = &mut app.sessions.write().await;
        let session = Arc::make_mut(
            sessions
                .get_mut(&query.id)
                .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?,
        );

        match query.action {
            SessionAction::Start => {
                {
                    let app = app.clone();
                    let duration = session.contest.duration;
                    tokio::task::spawn(async move {
                        tokio::time::sleep(duration.try_into().expect("invalid contest duration"))
                            .await;

                        let sessions = &mut app.sessions.write().await;
                        let session = sessions.get_mut(&query.id).unwrap();

                        Arc::make_mut(session).end(&app.db).await.ok();
                    });
                }

                session.start(&app.db).await?
            }
            SessionAction::End => session.end(&app.db).await?,
        }
    }

    let sessions = &mut app.sessions.read().await;
    let session = sessions
        .get(&query.id)
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

    tx.send(sessions.values().cloned().collect())?;

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
    contests: Vec<Arc<Contest>>,
    more: bool,
}

async fn contests(
    State(app): State<App>,
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
struct CreateSession {
    idx: usize,
}

async fn create_session(
    State(app): State<App>,
    Extension(tx): Extension<Arc<Sender<Vec<Arc<Session>>>>>,
    Query(CreateSession { idx }): Query<CreateSession>,
) -> AppResult<Response> {
    let contest = app.contests[idx - 1].clone();
    let session = Session::new(&app.db, contest).await?;

    app.sessions
        .write()
        .await
        .insert(session.id, Arc::new(session));

    tx.send(app.sessions.read().await.values().cloned().collect())?;

    Ok(Response::builder()
        .header("HX-Trigger", "reloadSessions")
        .body("Contest session created".into())?)
}

#[derive(Template)]
#[template(path = "admin/user_table.html")]
struct UserTable {
    page: usize,
    admins: Vec<i64>,
    users: Vec<User>,
    more: bool,
}

async fn users(
    State(app): State<App>,
    Query(Pagination { page }): Query<Pagination>,
) -> AppResult<UserTable> {
    let offset = 10 * (page - 1) as i64;

    let admins = sqlx::query!("SELECT id FROM admins;")
        .fetch(app.db.pool())
        .map(|res| res.map(|record| record.id))
        .collect::<Result<_, _>>()
        .await?;

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
        admins,
        users,
        more: count > page * 10,
    })
}

#[derive(Debug, Deserialize)]
struct UserQuery {
    id: i64,
}

async fn delete_user(
    auth_session: AuthSession,
    State(app): State<App>,
    Query(UserQuery { id }): Query<UserQuery>,
) -> AppResult<StatusCode> {
    let user = sqlx::query!("SELECT * FROM users WHERE id = ?;", id)
        .fetch_optional(app.db.pool())
        .await?
        .map(|user| User::new(user.id, &user.username, &user.password))
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

    if auth_session
        .backend
        .has_perm(&user, Permissions::ADMIN)
        .await?
    {
        Ok(StatusCode::UNAUTHORIZED)
    } else {
        sqlx::query!("DELETE FROM users WHERE id = ?;", id)
            .execute(app.db.pool())
            .await?;
        Ok(StatusCode::OK)
    }
}
