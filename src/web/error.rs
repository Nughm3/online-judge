use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use color_eyre::Report;

#[derive(Debug)]
pub enum AppError {
    Report(Report),
    StatusCode(StatusCode),
}

pub type AppResult<T> = std::result::Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Report(report) => {
                (StatusCode::INTERNAL_SERVER_ERROR, report.to_string()).into_response()
            }
            AppError::StatusCode(code) => code.into_response(),
        }
    }
}

impl<E: Into<Report>> From<E> for AppError {
    fn from(error: E) -> Self {
        AppError::Report(error.into())
    }
}

impl AppError {
    pub fn into_report(self) -> Report {
        match self {
            AppError::Report(report) => report,
            AppError::StatusCode(code) => Report::msg(format!("status code {code}")),
        }
    }
}
