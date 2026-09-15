use crate::dto::auth::{AuthResponse, AuthSuccess, CognitoTokenError, CognitoTokens, LoginUrl};
use crate::util::config::Config;
use crate::util::error::ServiceResult;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use shared::error::AppError;
use std::sync::Arc;
use url::Url;

const SCOPES: &str = "openid email profile";

pub struct AuthService {
    config: Arc<Config>,
}

impl AuthService {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn login_url(&self, redirect_uri: &str) -> ServiceResult<LoginUrl> {
        let domain = &self.config.domain;

        let (code_verifier, code_challenge) = Self::generate_pkce();

        let mut url = Url::parse(&format!("{domain}/login")).map_err(|e| {
            println!("{:?}", e);
            AppError::ServerError
        })?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", SCOPES)
            .append_pair("code_challenge", &code_challenge)
            .append_pair("prompt", "none")
            .append_pair("code_challenge_method", "S256");

        Ok(LoginUrl {
            url: url.to_string(),
            code_verifier,
        })
    }

    pub fn logout_url(&self, redirect_uri: &str) -> ServiceResult<String> {
        let domain = &self.config.domain;

        let mut url = Url::parse(&format!("{domain}/logout")).map_err(|e| {
            println!("{:?}", e);
            AppError::ServerError
        })?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("logout_uri", redirect_uri);

        Ok(url.to_string())
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> ServiceResult<AuthResponse> {
        let domain = &self.config.domain;

        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| {
                println!("{:?}", e);
                AppError::ServerError
            })?;

        let response = http_client
            .post(format!("{domain}/oauth2/token"))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", self.config.client_id.as_str()),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                AppError::ServerError
            })?;

        let success = response.status().is_success();
        let body = response.bytes().await.map_err(|e| {
            println!("{:?}", e);
            AppError::ServerError
        })?;

        if !success {
            let message = serde_json::from_slice::<CognitoTokenError>(&body)
                .map(|err| err.error_description.unwrap_or(err.error))
                .unwrap_or_else(|_| "failed to exchange authorization code".into());

            return Err(AppError::HttpMessage(400, message));
        }

        let tokens = serde_json::from_slice::<CognitoTokens>(&body).map_err(|e| {
            println!("{:?}", e);
            AppError::ServerError
        })?;

        Ok(AuthResponse::Success(AuthSuccess::new(
            tokens.access_token,
            tokens.id_token,
            tokens.refresh_token,
        )))
    }

    fn generate_pkce() -> (String, String) {
        let mut verifier_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut verifier_bytes);

        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
        let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));

        (code_verifier, code_challenge)
    }
}
