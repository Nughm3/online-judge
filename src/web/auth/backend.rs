use axum::async_trait;
use axum_login::{AuthnBackend, AuthzBackend, UserId};
use sqlx::SqlitePool;

use super::{Credentials, Permissions, User};
use crate::web::database::Database;

pub type AuthSession = axum_login::AuthSession<Backend>;

#[derive(Debug, Clone)]
pub struct Backend(SqlitePool);

impl Backend {
    pub fn new(db: &Database) -> Self {
        Backend(db.pool().clone())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.0
    }
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = sqlx::Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user: Option<Self::User> = sqlx::query_as!(
            Self::User,
            "SELECT * FROM users WHERE username = ?;",
            creds.username
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(user
            .filter(|user| password_auth::verify_password(creds.password, user.password()).is_ok()))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        Ok(
            sqlx::query_as!(Self::User, "SELECT * FROM users WHERE id = ?;", user_id)
                .fetch_optional(self.pool())
                .await?,
        )
    }
}

#[async_trait]
impl AuthzBackend for Backend {
    type Permission = Permissions;

    async fn has_perm(
        &self,
        user: &Self::User,
        perm: Self::Permission,
    ) -> Result<bool, Self::Error> {
        Ok(if perm == Permissions::ADMIN {
            let user_id = user.id();
            sqlx::query!("SELECT * FROM admins WHERE user_id = ?;", user_id)
                .fetch_optional(self.pool())
                .await?
                .is_some()
        } else {
            true
        })
    }
}
