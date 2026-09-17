use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};

#[derive(Serialize, Debug, ToSchema)]
pub struct AuthChallenge {
    pub session: Option<String>,
    pub challenge: String,
    pub parameters: Option<HashMap<String, String>>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct AuthSuccess {
    pub access_token: String,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl AuthSuccess {
    pub fn new(
        access_token: String,
        id_token: Option<String>,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            access_token,
            id_token,
            refresh_token,
        }
    }
}

#[derive(Serialize, Debug, ToSchema)]
#[serde(untagged)]
pub enum AuthResponse {
    Success(AuthSuccess),
    Challenge(AuthChallenge),
    UserStep,
}

pub struct LoginUrl {
    pub url: String,
    pub code_verifier: String,
}

#[derive(Deserialize)]
pub(crate) struct CognitoTokens {
    pub(crate) access_token: String,
    pub(crate) id_token: Option<String>,
    pub(crate) refresh_token: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct CognitoTokenError {
    pub(crate) error: String,
    #[serde(default)]
    pub(crate) error_description: Option<String>,
}

#[derive(Deserialize, IntoParams)]
pub struct LoginQuery {
    pub redirect_uri: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub url: String,
    pub code_verifier: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CallbackRequest {
    pub code: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

#[derive(Deserialize, IntoParams)]
pub struct LogoutQuery {
    pub redirect_uri: String,
}

#[derive(Serialize, ToSchema)]
pub struct LogoutResponse {
    pub url: String,
}
