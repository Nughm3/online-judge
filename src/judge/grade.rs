use super::{run::TestResult, *};
use crate::contest::Task;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GradedTask {
    pub verdict: Verdict,
    pub score: u32,
    pub subtasks: Vec<GradedSubtask>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GradedSubtask {
    pub verdict: Verdict,
    pub score: u32,
    pub tests: Vec<GradedTest>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GradedTest {
    pub verdict: Verdict,
    pub score: u32,
    pub resource_usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    CompileError,
    RuntimeError,
    WrongAnswer,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    PartialScore,
    Accepted,
}

impl Verdict {
    pub fn fmt_colored(&self) -> impl fmt::Display + '_ {
        let paint = match self {
            Verdict::CompileError | Verdict::RuntimeError => Paint::yellow,
            Verdict::WrongAnswer => Paint::red,
            Verdict::TimeLimitExceeded | Verdict::MemoryLimitExceeded => Paint::magenta,
            Verdict::PartialScore => Paint::blue,
            Verdict::Accepted => Paint::green,
        };

        paint(self).bold()
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::CompileError => "Compile Error",
            Verdict::RuntimeError => "Runtime Error",
            Verdict::WrongAnswer => "Wrong Answer",
            Verdict::TimeLimitExceeded => "Time Limit Exceeded",
            Verdict::MemoryLimitExceeded => "Memory Limit Exceeded",
            Verdict::PartialScore => "Partial Score",
            Verdict::Accepted => "Accepted",
        }
        .fmt(f)
    }
}

#[derive(Debug, Error)]
#[error("invalid verdict: {0}")]
pub struct InvalidVerdict(String);

impl FromStr for Verdict {
    type Err = InvalidVerdict;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "compile error" => Verdict::CompileError,
            "runtime error" => Verdict::RuntimeError,
            "wrong answer" => Verdict::WrongAnswer,
            "time limit exceeded" => Verdict::TimeLimitExceeded,
            "memory limit exceeded" => Verdict::MemoryLimitExceeded,
            "partial score" => Verdict::PartialScore,
            "accepted" => Verdict::Accepted,
            _ => return Err(InvalidVerdict(s.to_owned())),
        })
    }
}

pub fn grade(task: &Task, results: &[TestResult]) -> GradedTask {
    let mut grade = GradedTask {
        verdict: Verdict::Accepted,
        score: 0,
        subtasks: Vec::with_capacity(task.subtasks.len()),
    };

    let mut iter = results.iter();

    for subtask in task.subtasks.iter() {
        let mut subtask_grade = GradedSubtask {
            verdict: Verdict::Accepted,
            score: 0,
            tests: Vec::with_capacity(subtask.tests),
        };

        for TestResult {
            verdict,
            resource_usage,
        } in iter.by_ref().copied().take(subtask.tests)
        {
            let score = if let Verdict::Accepted = verdict {
                1
            } else {
                0
            };

            subtask_grade.score += score;
            subtask_grade.verdict = subtask_grade.verdict.min(verdict);
            subtask_grade.tests.push(GradedTest {
                verdict,
                score,
                resource_usage,
            })
        }

        grade.verdict = grade.verdict.min(subtask_grade.verdict);
        grade.score += subtask_grade.score;
        grade.subtasks.push(subtask_grade);
    }

    grade
}
