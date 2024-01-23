use std::io::ErrorKind;

use rayon::prelude::*;

use super::*;
use crate::contest::{ContestResourceLimits, Task, Test};

const MEMORY_USAGE_EPSILON: u64 = 1000;
const TIME_ELAPSED_EPSILON: f64 = 0.1;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct TestResult {
    pub verdict: Verdict,
    pub resource_usage: Option<ResourceUsage>,
}

#[tracing::instrument(skip(task), err)]
pub fn run(
    config: &Config,
    submission: Submission,
    task: &Task,
    rlimits: ContestResourceLimits,
) -> JudgeResult<Vec<TestResult>> {
    let Some(language) = config
        .languages
        .iter()
        .find(|language| language.name == submission.language)
    else {
        return Err(JudgeError::UnknownLanguage(submission.language));
    };

    let sandbox = Sandbox::new()?;
    sandbox.write(&language.filename, submission.code)?;

    if let Some(command) = &language.build {
        build(&sandbox, command, rlimits.build)?;
    } else {
        tracing::debug!("skipping build (no build step)");
    }

    let mut verdicts: Vec<_> = task
        .tests
        .par_iter()
        .enumerate()
        .map(|(idx, test_case)| {
            let verdict = test(
                &sandbox,
                &language.run,
                rlimits.run,
                test_case,
                (idx + 1, task.tests.len()),
            )?;
            Ok((idx, verdict))
        })
        .collect::<JudgeResult<_>>()?;

    verdicts.par_sort_by_key(|(idx, _)| *idx);
    Ok(verdicts
        .into_iter()
        .map(|(_, test_result)| test_result)
        .collect())
}

#[tracing::instrument(err)]
fn build(sandbox: &Sandbox, command: &Command, rlimits: ResourceLimits) -> JudgeResult<()> {
    tracing::debug!("starting build");
    let output = sandbox.build(command, rlimits)?;

    if !output.exit_status.success() {
        tracing::error!("build failed");

        let stderr = std::str::from_utf8(&output.stderr).unwrap_or_default();

        if let Ok(stdout) = std::str::from_utf8(&output.stdout) {
            if !stdout.is_empty() {
                tracing::debug!("stdout: {stdout}");
            }
        }

        Err(JudgeError::CompileError(stderr.to_owned()))
    } else {
        let duration = output.resource_usage.user_time + output.resource_usage.user_time;
        tracing::debug!("build completed in {:.03}", duration.as_seconds_f64());
        Ok(())
    }
}

#[tracing::instrument(skip(sandbox, command, rlimits, test, test_count), err)]
fn test(
    sandbox: &Sandbox,
    command: &Command,
    rlimits: ResourceLimits,
    test: &Test,
    (test_number, test_count): (usize, usize),
) -> JudgeResult<TestResult> {
    let output = match sandbox.run(command, test.input.as_bytes(), rlimits) {
        Ok(output) => output,
        Err(e) => {
            if let ErrorKind::BrokenPipe = e.kind() {
                let verdict = Verdict::RuntimeError;
                tracing::trace!("[{test_number}/{test_count}] {}", verdict.fmt_colored());
                return Ok(TestResult {
                    verdict,
                    resource_usage: None,
                });
            } else {
                return Err(JudgeError::Io(e));
            }
        }
    };

    let stdout = std::str::from_utf8(&output.stdout)?;

    let verdict = if output.exit_status.success() {
        if stdout.trim() == test.output.trim() {
            Verdict::Accepted
        } else {
            Verdict::WrongAnswer
        }
    } else if output.exit_status.code().is_none() {
        let (memory_usage, memory_limit) =
            (output.resource_usage.memory_bytes, rlimits.memory_bytes);

        let (time_elapsed, time_limit) = (
            output.resource_usage.total_time().as_seconds_f64(),
            rlimits.cpu_seconds as f64,
        );

        if memory_usage > memory_limit || memory_limit - memory_usage <= MEMORY_USAGE_EPSILON {
            Verdict::MemoryLimitExceeded
        } else if time_elapsed > time_limit || time_limit - time_elapsed <= TIME_ELAPSED_EPSILON {
            Verdict::TimeLimitExceeded
        } else {
            panic!("process was killed by signal but it did not exceed the time or memory limit");
        }
    } else {
        let stderr = std::str::from_utf8(&output.stderr)?;
        tracing::trace!("stdout: {stdout}, stderr: {stderr}",);
        return Err(JudgeError::RuntimeError(stderr.to_owned()));
    };

    tracing::trace!("[{test_number}/{test_count}] {}", verdict.fmt_colored());
    Ok(TestResult {
        verdict,
        resource_usage: Some(output.resource_usage),
    })
}
