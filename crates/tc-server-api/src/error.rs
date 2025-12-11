use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tc_core::database::SqlError;

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    BadRequest(String),
    Database(SqlError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        (status, Json(serde_json::json!({"error": msg}))).into_response()
    }
}
