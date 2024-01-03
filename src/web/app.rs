use std::{collections::HashMap, sync::Arc};

use askama::Template;
use axum::{extract::State, routing::get, Router};
use serde::Deserialize;
use tokio::sync::Mutex;

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
    pub contests: Vec<Contest>,
    pub sessions: Mutex<HashMap<i64, Session>>,
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
            .lock()
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
}

async fn navbar(auth_session: AuthSession) -> Navbar {
    Navbar {
        user: auth_session.user,
    }
}
