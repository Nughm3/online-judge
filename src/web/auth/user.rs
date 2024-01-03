use std::fmt;

use axum_login::AuthUser;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct User {
    pub(super) id: i64,
    pub(super) username: String,
    pub(super) password: String,
}

impl User {
    pub fn new(id: i64, username: &str, password: &str) -> Self {
        User {
            id,
            username: username.to_owned(),
            password: password.to_owned(),
        }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub(super) fn password(&self) -> &str {
        &self.password
    }
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password.as_bytes()
    }
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Permissions {
    admin: bool,
}

impl Permissions {
    pub const ADMIN: Permissions = Permissions { admin: true };
}
