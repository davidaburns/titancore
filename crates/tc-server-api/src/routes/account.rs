use crate::{error::ApiError, sql::accounts};
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tc_core::{
    crypto::{defines::Salt, srp6},
    database::{DatabaseHandle, Result as SqlResult},
};

const ACCOUNT_USERNAME_MAX_LENGTH: usize = 16;
const ACCOUNT_PASSWORD_MAX_LENGTH: usize = 16;

#[derive(Debug, Deserialize)]
pub struct CreateAccount {
    pub username: String,
    pub password: String,
    pub email: String,
}

pub async fn create_account(
    State(db): State<Arc<DatabaseHandle>>,
    Json(input): Json<CreateAccount>,
) -> Result<StatusCode, ApiError> {
    if input.username.len() > ACCOUNT_USERNAME_MAX_LENGTH {
        return Err(ApiError::BadRequest("Username is too long".to_string()));
    }
    if input.password.len() > ACCOUNT_PASSWORD_MAX_LENGTH {
        return Err(ApiError::BadRequest("Password is too long".to_string()));
    }

    if account_exists_by_username(&input.username, &db)
        .await
        .map_err(|e| ApiError::Database(e))?
    {
        return Err(ApiError::BadRequest("Username already in use".to_string()));
    }

    let salt = Salt::randomized();
    let verifier = srp6::calculate_password_verifier(
        &input.username,
        &input.password,
        &salt,
        &srp6::Generator::default(),
        &srp6::LargeSafePrime::default(),
    );

    db.execute(
        accounts::ACCOUNT_CREATE,
        &[
            &input.username,
            &salt.as_bytes_le().to_vec(),
            &verifier.as_bytes_le().to_vec(),
            &input.email,
            &input.email,
        ],
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    db.execute(accounts::ACCOUNT_INIT_REALM_CHARACTERS, &[])
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(StatusCode::CREATED)
}

async fn account_exists_by_username(
    username: &String,
    db: &Arc<DatabaseHandle>,
) -> SqlResult<bool> {
    let exists: bool = db
        .query_scalar(accounts::ACCOUNT_EXISTS_BY_USERNAME, &[username])
        .await?;

    Ok(exists)
}
