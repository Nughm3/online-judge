use std::{collections::HashMap, sync::Arc};

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware::map_response_with_state,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_login::{login_required, AuthzBackend};
use serde::Deserialize;
use time::macros::format_description;
use tokio::sync::RwLock;

use super::{
    auth::{AuthSession, Backend, Permissions, User},
    database::Database,
    session::Session,
};
use crate::{contest::*, judge::Config as JudgeConfig};

mod contest;
mod leaderboard;
mod submit;

const LANGUAGE_COOKIE: &str = "preferred-language";

#[derive(Debug)]
pub struct App {
    pub db: Database,
    pub contests: Vec<Arc<Contest>>,
    pub sessions: RwLock<HashMap<i64, Session>>,
    pub judge_config: JudgeConfig,
}

pub fn router(app: Arc<App>) -> Router {
    let contest = {
        use self::{contest::*, leaderboard::*, submit::*};

        async fn ensure_contest_started(
            auth_session: AuthSession,
            State(app): State<Arc<App>>,
            Path(params): Path<HashMap<String, String>>,
            response: Response,
        ) -> Response {
            if let Some(Ok(session_id)) = params.get("session_id").map(|s| s.parse()) {
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
                        return response;
                    }
                }
            }

            StatusCode::NOT_FOUND.into_response()
        }

        Router::new()
            .route("/submit/:task_id", get(submissions).post(submit))
            .route("/task/:task_id", get(task))
            .route_layer(login_required!(Backend, login_url = "/login"))
            .route("/leaderboard", get(leaderboard))
            .route_layer(map_response_with_state(app.clone(), ensure_contest_started))
            .route("/", get(contest))
    };

    Router::new()
        .nest("/contest/:session_id", contest)
        .route("/", get(index))
        .route("/navbar", get(navbar))
        .with_state(app)
}

#[derive(Debug, Deserialize)]
pub struct ContestNavigation {
    session_id: i64,
    task_id: i64,
}

#[derive(Debug, Deserialize)]
pub(super) struct Pagination {
    pub(super) page: usize,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexPage {
    sessions: Vec<Session>,
}

async fn index(State(app): State<Arc<App>>) -> IndexPage {
    IndexPage {
        sessions: app
            .sessions
            .read()
            .await
            .values()
            .filter(|session| session.end.is_none())
            .cloned()
            .collect(),
    }
}

#[derive(Template)]
#[template(path = "navbar.html")]
struct Navbar {
    user: Option<User>,
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
    State(app): State<Arc<App>>,
    auth_session: AuthSession,
    session: Option<Query<NavbarQuery>>,
) -> Result<Navbar, StatusCode> {
    let contest_info = if let Some(Query(NavbarQuery { session_id })) = session {
        let sessions = &app.sessions.read().await;
        let session = sessions.get(&session_id).ok_or(StatusCode::NOT_FOUND)?;
        Some(ContestInfo {
            session_id,
            name: session.contest.name.clone(),
            end: session.start.map(|start| {
                let end = start + session.contest.duration;
                let format = format_description!(
                    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
                );
                end.format(&format)
                    .expect("failed to format contest end time")
            }),
        })
    } else {
        None
    };

    Ok(Navbar {
        user: auth_session.user,
        contest_info,
    })
}
