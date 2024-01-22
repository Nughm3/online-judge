use std::{collections::HashMap, sync::Arc};

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware::map_response_with_state,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum_login::{login_required, AuthzBackend};
use axum_typed_multipart::TypedMultipart;
use serde::Deserialize;
use time::OffsetDateTime;
use tokio_stream::StreamExt;
use tower_cookies::{Cookie, Cookies};

use super::{App, Pagination};
use crate::{
    contest::*,
    judge::{GradedTask, JudgeError, Submission, Verdict},
    web::{
        auth::{AuthSession, Backend, Permissions},
        error::*,
        session::Session,
    },
};

const LANGUAGE_COOKIE: &str = "language";

pub fn router(app: Arc<App>) -> Router<Arc<App>> {
    async fn ensure_contest_started(
        auth_session: AuthSession,
        State(app): State<Arc<App>>,
        Path(params): Path<HashMap<String, String>>,
        response: Response,
    ) -> Response {
        if let Some(Ok(session_id)) = params.get("session_id").map(|s| s.parse()) {
            if let Some(session) = app.sessions.read().await.get(&session_id) {
                let admin = if let Some(user) = auth_session.user {
                    auth_session
                        .backend
                        .has_perm(&user, Permissions::ADMIN)
                        .await
                        .unwrap_or_default()
                } else {
                    false
                };

                if session.start.is_some() || admin {
                    return response;
                }
            }
        }

        StatusCode::NOT_FOUND.into_response()
    }

    Router::new()
        .route("/submit/:task_id", get(submissions).post(submit))
        .route("/task/:task_id", get(task))
        .route_layer(login_required!(Backend, login_url = "/login"))
        .route("/leaderboard", get(leaderboard))
        .route("/leaderboard_table", get(leaderboard_table))
        .route_layer(map_response_with_state(app.clone(), ensure_contest_started))
        .route("/", get(contest))
}

async fn get_session(app: Arc<App>, id: i64) -> Option<Session> {
    app.sessions.read().await.get(&id).cloned()
}

async fn get_contest(app: Arc<App>, id: i64) -> Option<Arc<Contest>> {
    get_session(app, id).await.map(|session| session.contest)
}

#[derive(Template)]
#[template(path = "contest/contest.html")]
struct ContestPage {
    session_id: i64,
    contest: Arc<Contest>,
    started: bool,
    logged_in: bool,
}

async fn contest(
    auth_session: AuthSession,
    State(app): State<Arc<App>>,
    Path(session_id): Path<i64>,
) -> Result<ContestPage, StatusCode> {
    let session = get_session(app.clone(), session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(ContestPage {
        session_id,
        contest: session.contest,
        started: session.start.is_some(),
        logged_in: auth_session.user.is_some(),
    })
}

#[derive(Template)]
#[template(path = "contest/leaderboard.html")]
struct Leaderboard {
    contest_name: String,
    session_id: i64,
}

async fn leaderboard(
    State(app): State<Arc<App>>,
    Path(session_id): Path<i64>,
) -> AppResult<Leaderboard> {
    let contest = get_contest(app.clone(), session_id)
        .await
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

    Ok(Leaderboard {
        contest_name: contest.name.clone(),
        session_id,
    })
}

#[derive(Template)]
#[template(path = "contest/leaderboard_table.html")]
struct LeaderboardTable {
    session_id: i64,
    page: usize,
    rankings: Vec<(String, i64, u32)>,
    more: bool,
}

async fn leaderboard_table(
    State(app): State<Arc<App>>,
    Path(session_id): Path<i64>,
    Query(Pagination { page }): Query<Pagination>,
) -> AppResult<LeaderboardTable> {
    let offset = 10 * (page - 1) as i64;

    let rankings: Vec<_> = sqlx::query!(
        "SELECT users.username, users.id, SUM(max_score) AS total_score
         FROM (
           SELECT submissions.user_id, submissions.task, MAX(submissions.score) AS max_score
           FROM submissions
           WHERE submissions.session_id = ?
           GROUP BY submissions.user_id, submissions.task
         ) subquery
         JOIN users ON subquery.user_id = users.id
         GROUP BY users.username
         ORDER BY total_score DESC
         LIMIT 10 OFFSET ?;",
        session_id,
        offset
    )
    .fetch(app.db.pool())
    .map(|res| res.map(|rank| (rank.username, rank.id, rank.total_score as u32)))
    .collect::<Result<_, _>>()
    .await?;

    let count = sqlx::query!("SELECT COUNT(DISTINCT user_id) AS count FROM submissions;")
        .fetch_one(app.db.pool())
        .await?
        .count as usize;

    Ok(LeaderboardTable {
        session_id,
        page,
        rankings,
        more: count > page * 10,
    })
}

#[derive(Debug, Deserialize)]
struct ContestNavigation {
    session_id: i64,
    task_id: i64,
}

#[derive(Template)]
#[template(path = "contest/task.html")]
struct TaskPage {
    session_id: i64,
    contest_name: String,
    task_id: i64,
    has_prev: bool,
    has_next: bool,
    task: Task,
}

async fn task(
    State(app): State<Arc<App>>,
    Path(ContestNavigation {
        session_id,
        task_id,
    }): Path<ContestNavigation>,
) -> Result<TaskPage, StatusCode> {
    let contest = get_contest(app, session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let task = contest
        .tasks
        .get(task_id as usize - 1)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(TaskPage {
        session_id,
        contest_name: contest.name.clone(),
        task_id,
        has_prev: task_id > 1,
        has_next: task_id < contest.tasks.len() as i64,
        task,
    })
}

#[derive(Template)]
#[template(path = "contest/submit.html")]
struct SubmitPage {
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

struct TaskReport {
    submission_id: i64,
    datetime: OffsetDateTime,
    verdict: Verdict,
    score: u32,
    compile_error: Option<String>,
    subtask_report: SubtaskReport,
}

struct SubtaskReport {
    scores: Vec<(Verdict, u32)>,
    overall: (Verdict, u32),
}

async fn submissions(
    auth_session: AuthSession,
    cookies: Cookies,
    State(app): State<Arc<App>>,
    Path(ContestNavigation {
        session_id,
        task_id,
    }): Path<ContestNavigation>,
) -> AppResult<SubmitPage> {
    let user_id = auth_session
        .user
        .map(|user| user.id())
        .ok_or(AppError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let session = get_session(app.clone(), session_id)
        .await
        .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

    let accepting_submissions = session.start.is_some() && session.end.is_none();

    let cooldown = session
        .user_cooldowns
        .get(&(user_id, task_id))
        .and_then(|cooldown| {
            let elapsed = OffsetDateTime::now_utc() - *cooldown;
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
                overall: (Verdict::Accepted, 0),
            },
        })
    })
    .collect::<Result<_, _>>()
    .await?;

    for report in reports.iter_mut() {
        let mut stream = sqlx::query!(
            "SELECT * FROM subtask_scores WHERE submission_id = ? ORDER BY subtask;",
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
        let (overall_verdict, overall_score) = &mut report.subtask_report.overall;

        while let Some((verdict, score)) = stream.try_next().await? {
            scores.push((verdict, score));
            *overall_verdict = (*overall_verdict).min(verdict);
            *overall_score += score;
        }
    }

    let overall = reports
        .iter()
        .map(|report| report.verdict)
        .min()
        .map(|verdict| (verdict, reports.iter().map(|report| report.score).sum()));

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
async fn submit(
    auth_session: AuthSession,
    cookies: Cookies,
    State(app): State<Arc<App>>,
    Path(ContestNavigation {
        session_id,
        task_id,
    }): Path<ContestNavigation>,
    TypedMultipart(submission): TypedMultipart<Submission>,
) -> AppResult<Redirect> {
    let user_id = auth_session
        .user
        .map(|user| user.id())
        .ok_or(AppError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let redirect_url = format!("/contest/{session_id}/submit/{task_id}");

    let now = OffsetDateTime::now_utc();

    let judge_result = {
        let session = get_session(app.clone(), session_id)
            .await
            .ok_or(AppError::StatusCode(StatusCode::NOT_FOUND))?;

        if session.end.is_some() {
            return Ok(Redirect::to(&redirect_url));
        }

        if let Some(previous) = session.user_cooldowns.get(&(user_id, task_id)) {
            if now - *previous < session.contest.cooldown {
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
    let score = grade.score as i64;

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
        compile_error
    )
    .execute(app.db.pool()).await?.last_insert_rowid();

    for (idx, subtask) in grade.subtasks.iter().enumerate() {
        let subtask_idx = idx as i64 + 1;
        let subtask_verdict = subtask.verdict.to_string();
        let subtask_score = subtask.score as i64;

        let subtask_id = sqlx::query!(
            "INSERT INTO subtask_scores (submission_id, subtask, verdict, score) VALUES (?, ?, ?, ?);",
            submission_id,
            subtask_idx,
            subtask_verdict,
            subtask_score
        )
        .execute(app.db.pool()).await?.last_insert_rowid();

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
                "INSERT INTO test_scores (subtask_id, test, memory, time, verdict, score) VALUES (?, ?, ?, ?, ?, ?);",
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

    app.sessions
        .write()
        .await
        .get_mut(&session_id)
        .unwrap()
        .user_cooldowns
        .insert((user_id, task_id), OffsetDateTime::now_utc());

    tracing::trace!("submission successfully judged and recorded");

    Ok(Redirect::to(&redirect_url))
}
