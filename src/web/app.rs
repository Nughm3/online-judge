use std::{collections::HashMap, convert::Infallible, sync::Arc};

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware::map_response_with_state,
    response::{sse::*, Response},
    routing::get,
    Router,
};
use axum_login::{login_required, AuthzBackend};
use serde::Deserialize;
use time::macros::format_description;
use tokio::sync::{watch, RwLock};
use tokio_stream::{wrappers::WatchStream, Stream, StreamExt};

use super::{
    auth::{AuthSession, Backend, Permissions, User},
    database::Database,
    error::{AppError, AppResult},
    session::Session,
};
use crate::{contest::*, judge::Config as JudgeConfig};

mod admin;
mod contest;
mod leaderboard;
mod submit;

#[derive(Debug, Clone)]
pub struct App {
    pub db: Database,
    pub contests: Vec<Arc<Contest>>,
    pub sessions: Arc<RwLock<HashMap<i64, Arc<Session>>>>,
    pub judge_config: Arc<JudgeConfig>,
}

pub fn router(app: App) -> Router {
    let contest = {
        use self::{contest::*, leaderboard::*, submit::*};

        #[derive(Deserialize)]
        struct Params {
            session_id: Option<i64>,
        }

        async fn ensure_contest_started(
            auth_session: AuthSession,
            State(app): State<App>,
            Path(Params { session_id }): Path<Params>,
            response: Response,
        ) -> Result<Response, StatusCode> {
            if let Some(session_id) = session_id {
                if let Some(session) = app.sessions.read().await.get(&session_id) {
                    let admin = if let Some(user) = auth_session.user {
                        auth_session
                            .backend
                            .has_perm(&user, Permissions::ADMIN)
                            .await
                            .unwrap_or_default()
                    } else {
                        false
                    };

                    if session.start.is_some() || admin {
                        return Ok(response);
                    }
                }
            }

            Err(StatusCode::NOT_FOUND)
        }

        Router::new()
            .route("/submit/:task_id", get(submissions).post(submit))
            .route("/task/:task_id", get(task))
            .route_layer(login_required!(Backend, login_url = "/login"))
            .route("/leaderboard", get(leaderboard))
            .route("/leaderboard/rankings", get(leaderboard_rankings))
            .route("/leaderboard/sse", get(leaderboard_sse))
            .route_layer(map_response_with_state(app.clone(), ensure_contest_started))
            .route("/", get(contest))
    };

    let (tx, rx) = watch::channel(());

    let router = Router::new()
        .nest("/contest/:session_id", contest)
        .route("/", get(move || async { IndexPage }))
        .route("/sessions", get(sessions))
        .route("/sessions/sse", get(move || sessions_sse(rx)))
        .route("/navbar", get(navbar))
        .with_state(app.clone());

    admin::router(app, Arc::new(tx)).merge(router)
}

#[derive(Debug, Deserialize)]
struct ContestNavigation {
    session_id: i64,
    task_id: i64,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexPage;

#[derive(Template)]
#[template(path = "sessions.html")]
struct Sessions {
    sessions: Vec<Arc<Session>>,
}

async fn sessions(State(app): State<App>) -> Sessions {
    Sessions {
        sessions: app
            .sessions
            .read()
            .await
            .iter()
            .map(|(_, session)| session.clone())
            .collect(),
    }
}

async fn sessions_sse(
    rx: watch::Receiver<()>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(WatchStream::new(rx).map(|_| Ok(Event::default().event("session"))))
        .keep_alive(KeepAlive::new())
}

#[derive(Template)]
#[template(path = "navbar.html")]
struct Navbar {
    user: Option<User>,
    admin: bool,
    contest_info: Option<ContestInfo>,
}

struct ContestInfo {
    session_id: i64,
    name: String,
    end: Option<String>,
}

#[derive(Deserialize)]
struct NavbarQuery {
    session_id: i64,
}

async fn navbar(
    auth_session: AuthSession,
    State(app): State<App>,
    session: Option<Query<NavbarQuery>>,
) -> AppResult<Navbar> {
    let contest_info = if let Some(Query(NavbarQuery { session_id })) = session {
        let sessions = &app.sessions.read().await;
        let session = sessions
            .get(&session_id)
            .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

        Some(ContestInfo {
            session_id,
            name: session.contest.name.clone(),
            end: session
                .end
                .is_none()
                .then(|| {
                    session.start.map(|start| {
                        let end = start + session.contest.duration;
                        let format = format_description!(
                            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
                        );

                        end.format(&format)
                            .expect("failed to format contest end time")
                    })
                })
                .flatten(),
        })
    } else {
        None
    };

    let admin = if let Some(user) = &auth_session.user {
        auth_session
            .backend
            .has_perm(user, Permissions::ADMIN)
            .await?
    } else {
        false
    };

    Ok(Navbar {
        user: auth_session.user,
        admin,
        contest_info,
    })
}
