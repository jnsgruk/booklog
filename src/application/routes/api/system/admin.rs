use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use tracing::{error, info};

use tower_cookies::Cookies;

use crate::application::auth::{AuthenticatedUser, get_session_from_cookies};
use crate::application::errors::{ApiError, AppError};
use crate::application::routes::support::is_datastar_request;
use crate::application::state::AppState;
use crate::domain::ids::{PasskeyCredentialId, UserId};
use crate::domain::registration_tokens::NewRegistrationToken;
use crate::infrastructure::auth::{generate_session_token, hash_token};

#[derive(Serialize)]
pub struct PasskeyResponse {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub(crate) async fn list_passkeys(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<Json<Vec<PasskeyResponse>>, StatusCode> {
    let passkeys = state
        .passkey_repo
        .list_by_user(auth_user.effective.id)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to list passkeys");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let responses: Vec<PasskeyResponse> = passkeys
        .into_iter()
        .map(|p| PasskeyResponse {
            id: i64::from(p.id),
            name: p.name,
            created_at: p.created_at,
            last_used_at: p.last_used_at,
        })
        .collect();

    Ok(Json(responses))
}

pub(crate) async fn delete_passkey(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(passkey_id): Path<PasskeyCredentialId>,
) -> Result<StatusCode, StatusCode> {
    // Verify the passkey belongs to the user
    let passkey = state.passkey_repo.get(passkey_id).await.map_err(|err| {
        error!(error = %err, %passkey_id, "failed to get passkey for deletion");
        StatusCode::NOT_FOUND
    })?;

    if passkey.user_id != auth_user.effective.id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Ensure the user has more than one passkey
    let all_passkeys = state
        .passkey_repo
        .list_by_user(auth_user.effective.id)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to list passkeys for deletion check");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if all_passkeys.len() <= 1 {
        return Err(StatusCode::CONFLICT);
    }

    state.passkey_repo.delete(passkey_id).await.map_err(|err| {
        error!(error = %err, %passkey_id, "failed to delete passkey");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::NO_CONTENT)
}

#[tracing::instrument(skip(state, auth_user, headers))]
pub(crate) async fn create_invite(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if !auth_user.real.is_admin {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let token_value = generate_session_token();
    let token_hash_value = hash_token(&token_value);
    let now = Utc::now();
    let expires_at = now + Duration::days(7);

    let new_token = NewRegistrationToken::new(token_hash_value, now, expires_at);
    state
        .registration_token_repo
        .insert(new_token)
        .await
        .map_err(AppError::from)?;

    let base_url = crate::base_url();
    let invite_url = format!("{base_url}/register/{token_value}");

    info!(user_id = %auth_user.real.id, "invite link created");

    if is_datastar_request(&headers) {
        use serde_json::Value;
        let signals = vec![
            ("_invite-url", Value::String(invite_url)),
            ("_invite-created", Value::Bool(true)),
        ];
        crate::application::routes::support::render_signals_json(&signals).map_err(ApiError::from)
    } else {
        Ok(Json(serde_json::json!({ "invite_url": invite_url })).into_response())
    }
}

#[tracing::instrument(skip(state, auth_user, cookies))]
pub(crate) async fn start_impersonation(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    cookies: Cookies,
    Path(target_user_id): Path<UserId>,
) -> Result<Response, StatusCode> {
    if !auth_user.real.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    if target_user_id == auth_user.real.id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify target user exists
    let target = state.user_repo.get(target_user_id).await.map_err(|err| {
        error!(error = %err, %target_user_id, "impersonation target user not found");
        StatusCode::NOT_FOUND
    })?;

    let session = get_session_from_cookies(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    state
        .session_repo
        .set_acting_as(session.id, Some(target_user_id))
        .await
        .map_err(|err| {
            error!(error = %err, "failed to set impersonation on session");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(
        admin_id = %auth_user.real.id,
        target_user_id = %target_user_id,
        target_username = %target.username,
        "impersonation started"
    );

    Ok(StatusCode::OK.into_response())
}

#[tracing::instrument(skip(state, auth_user, cookies))]
pub(crate) async fn stop_impersonation(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    cookies: Cookies,
) -> Result<Response, StatusCode> {
    if !auth_user.is_impersonating {
        return Err(StatusCode::BAD_REQUEST);
    }

    let session = get_session_from_cookies(&state, &cookies)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    state
        .session_repo
        .set_acting_as(session.id, None)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to clear impersonation on session");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(admin_id = %auth_user.real.id, "impersonation stopped");

    Ok(Redirect::to("/admin").into_response())
}
