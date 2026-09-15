use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Debug)]
pub struct AuthChallenge {
    pub session: Option<String>,
    pub challenge: String,
    pub parameters: Option<HashMap<String, String>>,
}

#[derive(Serialize, Debug)]
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

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum AuthResponse {
    Success(AuthSuccess),
    Challenge(AuthChallenge),
    UserStep,
}

/// A Cognito managed login URL along with the PKCE code verifier used to generate it.
/// The caller is responsible for keeping `code_verifier` (e.g. in a short-lived cookie or
/// session) and supplying it back when exchanging the authorization code for tokens.
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

#[derive(Deserialize)]
pub struct LoginQuery {
    pub redirect_uri: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub url: String,
    pub code_verifier: String,
}

/// `code_verifier` is generated and held by the caller (returned from `/login`), and
/// `redirect_uri` must match the one used there.
#[derive(Deserialize)]
pub struct CallbackRequest {
    pub code: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

#[derive(Deserialize)]
pub struct LogoutQuery {
    pub redirect_uri: String,
}

#[derive(Serialize)]
pub struct LogoutResponse {
    pub url: String,
}
