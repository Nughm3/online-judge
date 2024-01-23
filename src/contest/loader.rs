use std::{fs, io, path::Path};

use pulldown_cmark::{BrokenLink, Options, Parser};
use thiserror::Error;

use super::*;

#[derive(Debug, Error)]
pub enum LoadContestError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse YAML front matter: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("task description does not have front matter")]
    NoFrontmatter,
    #[error("no subtasks in task")]
    NoSubtasks,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContestFrontmatter {
    name: String,
    #[serde(rename = "tasks")]
    task_paths: Vec<String>,
    languages: Option<Vec<String>>,
    duration: Duration,
    #[serde(default = "defaults::cooldown")]
    cooldown: Duration,
    #[serde(default = "defaults::leaderboard_size")]
    leaderboard_size: usize,
    rlimits: ContestResourceLimits,
}

impl Contest {
    #[tracing::instrument(skip(path))]
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LoadContestError> {
        let path = path.as_ref();

        tracing::debug!("loading contest at path {}", path.display());
        let mut tasks = Vec::new();

        let input = fs::read_to_string(path.join("contest.md"))?;
        let (frontmatter, page) = extract_frontmatter::<ContestFrontmatter>(&input)?;
        let page = parse_markdown(&page);

        for task_path in frontmatter.task_paths {
            let path = path.join(task_path);
            if !path.is_dir() {
                return Err(LoadContestError::Io(io::Error::new(
                    io::ErrorKind::Other, // NotADirectory
                    "task is not a directory",
                )));
            }

            tasks.push(Task::load(&path)?);
        }

        Ok(Contest {
            name: frontmatter.name,
            path: path.to_path_buf(),
            page,
            tasks,
            languages: frontmatter.languages,
            duration: frontmatter.duration,
            cooldown: frontmatter.cooldown,
            leaderboard_size: frontmatter.leaderboard_size,
            rlimits: frontmatter.rlimits,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskFrontmatter {
    name: String,
    #[serde(default)]
    examples: Vec<Example>,
    subtasks: Vec<Subtask>,
    #[serde(default)]
    constraints: Vec<String>,
    #[serde(default)]
    difficulty: Option<Difficulty>,
}

impl Task {
    fn load(path: &Path) -> Result<Self, LoadContestError> {
        tracing::trace!("loading task at path {}", path.display());
        let input = fs::read_to_string(path.join("task.md"))?;

        let (frontmatter, page) = extract_frontmatter::<TaskFrontmatter>(&input)?;
        let page = parse_markdown(&page);

        if frontmatter.subtasks.is_empty() {
            return Err(LoadContestError::NoSubtasks);
        }

        let mut tests = Vec::new();
        let test_dir = path.join("tests");

        let mut n = 1;
        for (idx, subtask) in frontmatter.subtasks.iter().enumerate() {
            for _ in 0..subtask.tests {
                let (Ok(input), Ok(output)) = (
                    fs::read_to_string(test_dir.join(format!("{n}.in"))),
                    fs::read_to_string(test_dir.join(format!("{n}.out"))),
                ) else {
                    break;
                };

                n += 1;

                tests.push(Test {
                    subtask: idx + 1,
                    input,
                    output,
                });
            }
        }

        Ok(Task {
            name: frontmatter.name,
            page,
            examples: frontmatter.examples,
            subtasks: frontmatter.subtasks,
            tests,
            constraints: frontmatter.constraints,
            difficulty: frontmatter.difficulty,
        })
    }
}

fn extract_frontmatter<'a, T: Deserialize<'a>>(
    input: &'a str,
) -> Result<(T, String), LoadContestError> {
    let stripped = input
        .strip_prefix("---\n")
        .ok_or(LoadContestError::NoFrontmatter)?;

    let end = stripped
        .find("---\n")
        .ok_or(LoadContestError::NoFrontmatter)?;

    Ok((
        serde_yaml::from_str(&stripped[..end])?,
        input[end + 8..].to_owned(),
    ))
}

fn parse_markdown(input: &str) -> String {
    let mut html = String::new();

    let mut callback = |BrokenLink {
                            span,
                            link_type,
                            reference,
                        }| {
        tracing::warn!("broken '{link_type:?}' link to {reference} at {span:?}");
        None
    };

    let parser = Parser::new_with_broken_link_callback(input, Options::all(), Some(&mut callback));

    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

mod defaults {
    use time::Duration;

    pub fn cooldown() -> Duration {
        Duration::hours(1)
    }

    pub fn leaderboard_size() -> usize {
        100
    }
}
