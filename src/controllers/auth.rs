use crate::dto::auth::{
    AuthResponse, CallbackRequest, LoginQuery, LoginResponse, LogoutQuery, LogoutResponse,
};
use crate::util::cookies;
use crate::util::state::ServiceStateType;

use axum::Json;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use shared::response::ServiceResponse;

pub async fn login(
    State(state): ServiceStateType,
    Query(query): Query<LoginQuery>,
) -> ServiceResponse<LoginResponse> {
    state
        .auth_service
        .login_url(&query.redirect_uri)
        .map(|login_url| LoginResponse {
            url: login_url.url,
            code_verifier: login_url.code_verifier,
        })
        .into()
}

pub async fn callback(
    State(state): ServiceStateType,
    Json(payload): Json<CallbackRequest>,
) -> impl IntoResponse {
    let result = state
        .auth_service
        .exchange_code(&payload.code, &payload.code_verifier, &payload.redirect_uri)
        .await;

    let jar = result
        .as_ref()
        .map(cookies::jar_for_response)
        .unwrap_or_default();

    let response: ServiceResponse<AuthResponse> = result.into();

    (jar, response)
}

pub async fn logout(
    State(state): ServiceStateType,
    Query(query): Query<LogoutQuery>,
) -> impl IntoResponse {
    let jar = cookies::expired_jar();

    println!("{:#?}", jar);
    let response: ServiceResponse<LogoutResponse> = state
        .auth_service
        .logout_url(&query.redirect_uri)
        .map(|url| LogoutResponse { url })
        .into();

    (jar, response)
}
