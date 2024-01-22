use std::{collections::HashMap, sync::Arc};

use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Router,
};
use serde::Deserialize;
use time::macros::format_description;
use tokio::sync::RwLock;

use super::{
    auth::{AuthSession, User},
    database::Database,
    session::Session,
};
use crate::contest::*;

mod contest;

#[derive(Debug)]
pub struct App {
    pub db: Database,
    pub contests: Vec<Arc<Contest>>,
    pub sessions: RwLock<HashMap<i64, Session>>,
    pub judge_config: crate::judge::Config,
}

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .nest("/contest/:session_id", contest::router(app.clone()))
        .route("/", get(index))
        .route("/navbar", get(navbar))
        .with_state(app)
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
