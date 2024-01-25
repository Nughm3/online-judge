use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use askama::Template;
use axum::{
    self,
    error_handling::HandleErrorLayer,
    extract::Request,
    http::StatusCode,
    middleware::{from_fn, Next},
    BoxError,
};
use axum_login::{
    tower_sessions::{MemoryStore, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use tokio::{fs, net::TcpListener, sync::RwLock};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

use self::database::Database;
use crate::contest::Contest;

mod app;
mod auth;
mod database;
mod error;
mod session;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Config {
    pub server_address: SocketAddr,
    pub database_url: String,
    pub contest_dir: PathBuf,
    pub static_dir: String,
    pub judge_config_path: PathBuf,
}

#[tracing::instrument]
pub async fn serve(config: Config) -> error::AppResult<()> {
    let db = Database::new(&config.database_url).await?;
    tracing::debug!("database loaded at {}", &config.database_url);

    let auth_service = {
        let session_store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(axum_login::tower_sessions::Expiry::OnInactivity(
                time::Duration::days(1),
            ));

        let backend = auth::Backend::new(&db);

        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::BAD_REQUEST
            }))
            .layer(AuthManagerLayerBuilder::new(backend, session_layer).build())
    };

    let mut contest_paths = fs::read_dir(config.contest_dir).await?;
    let mut contests = Vec::new();
    while let Some(entry) = contest_paths.next_entry().await? {
        let contest = tokio::task::spawn_blocking(move || Contest::load(entry.path())).await??;
        contests.push(Arc::new(contest));
    }
    tracing::debug!("loaded {} contests", contests.len());

    let judge_config = {
        let judge_config_file = fs::read_to_string(&config.judge_config_path).await?;
        tracing::debug!(
            "loading judge config {}",
            config.judge_config_path.display()
        );
        toml::from_str(&judge_config_file)?
    };

    let app = app::router(app::App {
        db,
        contests,
        sessions: Arc::new(RwLock::new(HashMap::new())),
        judge_config,
    })
    .merge(auth::router())
    .nest_service("/static", ServeDir::new(config.static_dir))
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(from_fn(|request: Request, next: Next| async {
                #[derive(Template)]
                #[template(path = "not_found.html")]
                struct NotFound;

                let htmx = request.headers().contains_key("HX-Request");

                let mut response = next.run(request).await;
                if response.status() == StatusCode::NOT_FOUND && !htmx {
                    *response.body_mut() =
                        NotFound.render().expect("failed to render template").into();
                }

                response
            }))
            .layer(CookieManagerLayer::new())
            .layer(auth_service),
    );

    let listener = TcpListener::bind(config.server_address).await?;
    tracing::info!("listening on http://{}", listener.local_addr().unwrap());

    axum::serve(listener, app).await?;

    Ok(())
}
