use std::fmt;

use serde::Deserialize;

pub use self::{backend::*, router::router, user::*};

mod backend;
mod router;
mod user;

#[derive(Clone, Hash, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    username: String,
    password: String,
    next: Option<String>,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .field("next", &self.next)
            .finish()
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterCredentials {
    email: String,
    username: String,
    password: String,
    confirm: String,
    next: Option<String>,
}

impl fmt::Debug for RegisterCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegisterCredentials")
            .field("email", &self.email)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .field("confirm", &"[redacted]")
            .field("next", &self.next)
            .finish()
    }
}
