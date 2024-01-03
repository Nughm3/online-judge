use askama::Template;
use axum::{
    extract::Query,
    response::{IntoResponse, Redirect},
    routing::get,
    Form, Router,
};
use axum_login::AuthzBackend;
use serde::Deserialize;

use super::{backend::AuthSession, user::User, Credentials, Permissions, RegisterCredentials};
use crate::web::error::AppResult;

pub fn router() -> Router {
    Router::new()
        .route("/login", get(login_page).post(login))
        .route("/logout", get(logout))
        .route("/register", get(register_page).post(register))
}

#[derive(Debug, Deserialize)]
struct NextUrl {
    next: Option<String>,
}

#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate {
    retry: bool,
    next: Option<String>,
}

async fn login_page(
    auth_session: AuthSession,
    Query(NextUrl { next }): Query<NextUrl>,
) -> impl IntoResponse {
    if auth_session.user.is_some() {
        Redirect::to(&next.unwrap_or_else(|| String::from("/"))).into_response()
    } else {
        LoginTemplate { retry: false, next }.into_response()
    }
}

#[tracing::instrument(skip(auth_session))]
async fn login(
    mut auth_session: AuthSession,
    Form(creds): Form<Credentials>,
) -> AppResult<impl IntoResponse> {
    let user = match auth_session.authenticate(creds.clone()).await? {
        Some(user) => user,
        None => {
            return Ok(LoginTemplate {
                retry: true,
                next: creds.next,
            }
            .into_response());
        }
    };

    auth_session.login(&user).await?;

    let admin = auth_session
        .backend
        .has_perm(&user, Permissions::ADMIN)
        .await?;

    tracing::info!(
        "{} (ID: {}) logged in",
        if admin { "admin" } else { "user" },
        user.id()
    );

    let redirect = match &creds.next {
        Some(next) => Redirect::to(next),
        None if admin => Redirect::to("/admin"),
        None => Redirect::to("/"),
    };

    Ok(redirect.into_response())
}

#[tracing::instrument(skip(auth_session))]
async fn logout(mut auth_session: AuthSession) -> AppResult<Redirect> {
    if let Some(user) = auth_session.logout().await? {
        tracing::info!("user (ID: {}) logged out", user.id());
    } else {
        tracing::warn!("user not logged in");
    }

    Ok(Redirect::to("/login"))
}

#[derive(Template)]
#[template(path = "auth/register.html")]
struct RegisterTemplate {
    error: Option<String>,
    next: Option<String>,
}

async fn register_page(
    session: AuthSession,
    Query(NextUrl { next }): Query<NextUrl>,
) -> impl IntoResponse {
    if session.user.is_some() {
        Redirect::to(&next.unwrap_or_else(|| String::from("/"))).into_response()
    } else {
        RegisterTemplate { error: None, next }.into_response()
    }
}

#[tracing::instrument(skip(auth_session))]
async fn register(
    auth_session: AuthSession,
    Form(creds): Form<RegisterCredentials>,
) -> AppResult<impl IntoResponse> {
    if creds.password != creds.confirm {
        return Ok(RegisterTemplate {
            error: Some(String::from("Passwords do not match")),
            next: creds.next,
        }
        .into_response());
    }

    let password = password_auth::generate_hash(&creds.password);

    if sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE username = ?;",
        creds.username
    )
    .fetch_optional(auth_session.backend.pool())
    .await?
    .is_some()
    {
        return Ok(RegisterTemplate {
            error: Some(String::from("User already exists")),
            next: creds.next,
        }
        .into_response());
    }

    let result = sqlx::query!(
        "INSERT OR IGNORE INTO users (username, password) VALUES (?, ?);",
        creds.username,
        password,
    )
    .execute(auth_session.backend.pool())
    .await?;

    tracing::info!("user registered (ID: {})", result.last_insert_rowid());

    Ok(login(
        auth_session,
        Form(Credentials {
            username: creds.username,
            password: creds.password,
            next: creds.next,
        }),
    )
    .await
    .into_response())
}
