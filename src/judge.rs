use axum_typed_multipart::TryFromMultipart;
use serde::Deserialize;
use thiserror::Error;

pub use self::{grade::*, run::run, sandbox::*};

mod grade;
mod run;
mod sandbox;

pub type JudgeResult<T> = Result<T, JudgeError>;

#[derive(Debug, Error)]
pub enum JudgeError {
    #[error("failed to compile submission, stderr: {0}")]
    CompileError(String),
    #[error("failed to process submission output as UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown language: {0}")]
    UnknownLanguage(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    #[serde(alias = "language")]
    pub languages: Vec<Language>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, TryFromMultipart)]
pub struct Submission {
    pub code: String,
    pub language: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
pub struct Language {
    pub name: String,
    pub filename: String,
    pub build: Option<Command>,
    pub run: Command,
}
