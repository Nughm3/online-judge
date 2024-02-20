use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
};

use axum_typed_multipart::TypedMultipart;

use time::OffsetDateTime;
use tokio_stream::StreamExt;
use tower_cookies::{Cookie, Cookies};

use super::{App, ContestNavigation};
use crate::{
    judge::{GradedTask, JudgeError, Submission, Verdict},
    web::{auth::AuthSession, error::*, session::UserTask},
};

const LANGUAGE_COOKIE: &str = "preferred-language";

#[derive(Template)]
#[template(path = "contest/submit.html")]
pub struct SubmitPage {
    session_id: i64,
    task_id: i64,

    // Submission form
    accepting_submissions: bool,
    cooldown: Option<i64>,
    languages: Vec<String>,
    preferred_language: Option<String>,

    // Submission results
    reports: Vec<TaskReport>,
    overall: Option<(Verdict, u32)>,
}

pub struct TaskReport {
    submission_id: i64,
    datetime: OffsetDateTime,
    verdict: Verdict,
    score: u32,
    compile_error: Option<String>,
    subtask_report: SubtaskReport,
}

pub struct SubtaskReport {
    scores: Vec<(Verdict, u32, u32)>,
    overall: (Verdict, u32, u32),
}

pub async fn submissions(
    auth_session: AuthSession,
    cookies: Cookies,
    State(app): State<App>,
    Path(ContestNavigation {
        session_id,
        task_id,
    }): Path<ContestNavigation>,
) -> AppResult<SubmitPage> {
    let user_id = auth_session
        .user
        .map(|user| user.id())
        .ok_or(AppError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let sessions = &app.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

    let accepting_submissions = session.start.is_some() && session.end.is_none();

    let cooldown = session
        .users
        .get(&(user_id, task_id))
        .and_then(|user_task| {
            let elapsed = OffsetDateTime::now_utc() - user_task.cooldown;
            let contest_cooldown = session.contest.cooldown;
            (elapsed < contest_cooldown).then_some((contest_cooldown - elapsed).whole_seconds())
        });

    let languages = session.contest.languages.clone().unwrap_or_else(|| {
        app.judge_config
            .languages
            .iter()
            .map(|language| language.name.clone())
            .collect()
    });

    let preferred_language = cookies
        .get(LANGUAGE_COOKIE)
        .map(|cookie| cookie.value().to_owned());

    let mut reports: Vec<_> = sqlx::query!(
        "SELECT * FROM submissions WHERE user_id = ? AND session_id = ? AND task = ?;",
        user_id,
        session_id,
        task_id
    )
    .fetch(app.db.pool())
    .map(|res| {
        res.map(|submission| TaskReport {
            submission_id: submission.id,
            datetime: submission.datetime,
            verdict: submission.verdict.parse().expect("invalid verdict"),
            score: submission.score as u32,
            compile_error: submission.compile_error,
            subtask_report: SubtaskReport {
                scores: Vec::new(),
                overall: (Verdict::Accepted, 0, 0),
            },
        })
    })
    .collect::<Result<_, _>>()
    .await?;

    for report in reports.iter_mut() {
        let mut stream = sqlx::query!(
            "SELECT * FROM subtasks WHERE submission_id = ? ORDER BY subtask;",
            report.submission_id
        )
        .fetch(app.db.pool())
        .map(|res| {
            res.map(|score| {
                (
                    score.verdict.parse().expect("invalid verdict"),
                    score.score as u32,
                )
            })
        });

        let scores = &mut report.subtask_report.scores;
        let (overall_verdict, overall_score, overall_max) = &mut report.subtask_report.overall;

        let mut idx = 0;
        while let Some((verdict, score)) = stream.try_next().await? {
            // NOTE: this works on the assumption that 1 point is awarded for each correct test
            let max = session.contest.tasks[task_id as usize - 1].subtasks[idx].tests as u32;
            scores.push((verdict, score, max));
            *overall_verdict = (*overall_verdict).min(verdict);
            *overall_score += score;
            *overall_max += max;
            idx += 1;
        }
    }

    let overall = {
        let mut overall = None;

        for report in reports.iter() {
            if overall.is_none() || {
                let (verdict, score) = overall.unwrap();
                report.score > score || report.verdict > verdict
            } {
                overall = Some((report.verdict, report.score));
            }
        }

        overall
    };

    Ok(SubmitPage {
        session_id,
        task_id,
        accepting_submissions,
        cooldown,
        languages,
        preferred_language,
        reports,
        overall,
    })
}

#[tracing::instrument(skip(auth_session, app))]
pub async fn submit(
    auth_session: AuthSession,
    cookies: Cookies,
    State(app): State<App>,
    Path(ContestNavigation {
        session_id,
        task_id,
    }): Path<ContestNavigation>,
    TypedMultipart(submission): TypedMultipart<Submission>,
) -> AppResult<Redirect> {
    let user = auth_session
        .user
        .ok_or(AppError::StatusCode(StatusCode::UNAUTHORIZED))?;
    let user_id = user.id();

    let redirect_url = format!("/contest/{session_id}/submit/{task_id}");

    let now = OffsetDateTime::now_utc();

    let judge_result = {
        let session = app
            .sessions
            .read()
            .await
            .get(&session_id)
            .cloned()
            .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

        if session.end.is_some() {
            return Ok(Redirect::to(&redirect_url));
        }

        if let Some(previous) = session.users.get(&(user_id, task_id)) {
            if now - previous.cooldown < session.contest.cooldown {
                tracing::trace!("user (ID: {user_id}) attempted to submit but was on cooldown");
                return Ok(Redirect::to(&redirect_url));
            }
        }

        tracing::trace!("received submission from user (ID: {user_id}) for task {task_id} of contest session {session_id}");

        let config = app.judge_config.clone();
        let submission = submission.clone();
        let task = session
            .contest
            .tasks
            .get(task_id as usize - 1)
            .cloned()
            .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

        tokio::task::spawn_blocking(move || {
            use crate::judge;

            let verdicts = judge::run(&config, submission, &task, session.contest.rlimits)?;
            let grade = judge::grade(&task, &verdicts);

            Ok::<_, JudgeError>(grade)
        })
        .await?
    };

    let (grade, compile_error) = match judge_result {
        Ok(grade) => (grade, None),
        Err(JudgeError::CompileError(stderr)) => (
            GradedTask {
                verdict: Verdict::CompileError,
                score: 0,
                subtasks: Vec::new(),
            },
            Some(stderr),
        ),
        Err(e) => return Err(e.into()),
    };

    let verdict = grade.verdict.to_string();
    let score = grade.score;

    let submission_id = sqlx::query!(
        "INSERT INTO submissions (user_id, session_id, task, datetime, code, language, verdict, score, compile_error) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);",
        user_id,
        session_id,
        task_id,
        now,
        submission.code,
        submission.language,
        verdict,
        score,
        compile_error,
    )
    .execute(app.db.pool()).await?.last_insert_rowid();

    for (idx, subtask) in grade.subtasks.iter().enumerate() {
        let subtask_idx = idx as i64 + 1;
        let subtask_verdict = subtask.verdict.to_string();
        let subtask_score = subtask.score as i64;

        let subtask_id = sqlx::query!(
            "INSERT INTO subtasks (submission_id, subtask, verdict, score) VALUES (?, ?, ?, ?);",
            submission_id,
            subtask_idx,
            subtask_verdict,
            subtask_score
        )
        .execute(app.db.pool())
        .await?
        .last_insert_rowid();

        for (idx, test) in subtask.tests.iter().enumerate() {
            let test_idx = idx as i64 + 1;
            let test_verdict = test.verdict.to_string();
            let test_score = test.score as i64;

            let rusage = test.resource_usage;
            let memory = rusage.map(|rusage| rusage.memory_bytes as i64);
            let time = rusage.map(|rusage| {
                let duration = rusage.total_time();
                (duration.whole_milliseconds() as i64) + (duration.subsec_milliseconds() as i64)
            });

            sqlx::query!(
                "INSERT INTO tests (subtask_id, test, memory, time, verdict, score) VALUES (?, ?, ?, ?, ?, ?);",
                subtask_id,
                test_idx,
                memory,
                time,
                test_verdict,
                test_score
            )
            .execute(app.db.pool()).await?;
        }
    }

    cookies.add(Cookie::new(LANGUAGE_COOKIE, submission.language));

    let sessions = &mut app.sessions.write().await;
    let session = Arc::make_mut(sessions.get_mut(&session_id).unwrap());

    session
        .users
        .entry((user_id, task_id))
        .and_modify(|user_task| user_task.score = user_task.score.max(score))
        .or_insert_with(|| UserTask {
            score,
            cooldown: OffsetDateTime::now_utc(),
        });

    session.update_leaderboard(user.username(), user_id)?;

    tracing::trace!("submission successfully judged and recorded");

    Ok(Redirect::to(&redirect_url))
}
