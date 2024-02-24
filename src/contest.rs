use std::{fmt, path::PathBuf, str::FromStr};

use serde::Deserialize;
use serde_with::DeserializeFromStr;
use thiserror::Error;
use time::Duration;

use crate::judge::ResourceLimits;

mod loader;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Contest {
    pub name: String,
    pub path: PathBuf,
    pub page: String,
    pub tasks: Vec<Task>,
    pub duration: Duration,
    pub cooldown: Duration,
    pub leaderboard_size: usize,
    pub rlimits: ContestResourceLimits,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Task {
    pub name: String,
    pub page: String,
    pub examples: Vec<Example>,
    pub subtasks: Vec<Subtask>,
    pub constraints: Vec<String>,
    pub tests: Vec<Test>,
    pub difficulty: Option<Difficulty>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub input: String,
    pub output: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subtask {
    pub tests: usize,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Test {
    pub subtask: usize,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, DeserializeFromStr)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
        .fmt(f)
    }
}

#[derive(Debug, Error)]
#[error("invalid difficulty: {0}")]
pub struct InvalidDifficulty(String);

impl FromStr for Difficulty {
    type Err = InvalidDifficulty;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "easy" => Difficulty::Easy,
            "medium" => Difficulty::Medium,
            "hard" => Difficulty::Hard,
            _ => return Err(InvalidDifficulty(s.to_owned())),
        })
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContestResourceLimits {
    pub build: ResourceLimits,
    pub run: ResourceLimits,
}
