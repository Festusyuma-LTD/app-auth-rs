use crate::dto::auth::{
    AuthResponse, AuthSuccess, CognitoTokenError, CognitoTokens, CurrentUser, LoginUrl,
};
use crate::util::config::Config;
use crate::util::cookies;
use crate::util::error::ServiceResult;

use axum_extra::extract::cookie::CookieJar;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use moka::future::Cache;
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use shared::error::ServiceError as SharedError;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

const SCOPES: &str = "openid email profile";

const USER_CACHE_TTL: Duration = Duration::from_secs(30);
const USER_CACHE_CAPACITY: u64 = 10_000;

pub struct AuthService {
    config: Arc<Config>,
    http_client: reqwest::Client,
    user_cache: Cache<String, CurrentUser>,
}

impl AuthService {
    pub fn new(config: Arc<Config>) -> Self {
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("failed to build reqwest client");

        let user_cache = Cache::builder()
            .max_capacity(USER_CACHE_CAPACITY)
            .time_to_live(USER_CACHE_TTL)
            .build();

        Self {
            config,
            http_client,
            user_cache,
        }
    }

    pub fn login_url(&self, redirect_uri: &str) -> ServiceResult<LoginUrl> {
        let domain = &self.config.domain;

        let (code_verifier, code_challenge) = Self::generate_pkce();

        let mut url = Url::parse(&format!("{domain}/login")).map_err(|e| {
            println!("{:?}", e);
            SharedError::ServerError
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

    pub fn refresh_token_path(&self) -> String {
        format!("{}{}", self.config.base_path, cookies::REFRESH_TOKEN_ROUTE)
    }

    pub fn jar_for_response(&self, response: &AuthResponse) -> CookieJar {
        cookies::jar_for_response(
            response,
            &self.refresh_token_path(),
            self.config.token_expiration,
            self.config.refresh_expiration,
        )
    }

    pub fn expired_jar(&self) -> CookieJar {
        cookies::expired_jar(&self.refresh_token_path())
    }

    pub fn logout_url(&self, redirect_uri: &str) -> ServiceResult<String> {
        let domain = &self.config.domain;

        let mut url = Url::parse(&format!("{domain}/logout")).map_err(|e| {
            println!("{:?}", e);
            SharedError::ServerError
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

        let response = self
            .http_client
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
                SharedError::ServerError
            })?;

        let success = response.status().is_success();
        let body = response.bytes().await.map_err(|e| {
            println!("{:?}", e);
            SharedError::ServerError
        })?;

        if !success {
            let message = serde_json::from_slice::<CognitoTokenError>(&body)
                .map(|err| err.error_description.unwrap_or(err.error))
                .unwrap_or_else(|_| "failed to exchange authorization code".into());

            return Err(SharedError::HttpMessage(400, message));
        }

        let tokens = serde_json::from_slice::<CognitoTokens>(&body).map_err(|e| {
            println!("{:?}", e);
            SharedError::ServerError
        })?;

        Ok(AuthResponse::Success(AuthSuccess::new(
            tokens.access_token,
            tokens.id_token,
            tokens.refresh_token,
        )))
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> ServiceResult<AuthResponse> {
        let domain = &self.config.domain;

        let response = self
            .http_client
            .post(format!("{domain}/oauth2/token"))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", self.config.client_id.as_str()),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                SharedError::ServerError
            })?;

        let success = response.status().is_success();
        let body = response.bytes().await.map_err(|e| {
            println!("{:?}", e);
            SharedError::ServerError
        })?;

        if !success {
            let message = serde_json::from_slice::<CognitoTokenError>(&body)
                .map(|err| err.error_description.unwrap_or(err.error))
                .unwrap_or_else(|_| "failed to refresh token".into());

            return Err(SharedError::HttpMessage(400, message));
        }

        let tokens = serde_json::from_slice::<CognitoTokens>(&body).map_err(|e| {
            println!("{:?}", e);
            SharedError::ServerError
        })?;

        // Cognito's refresh grant doesn't rotate the refresh token by default, so the
        // response omits it — fall back to the one that was used to make this call.
        let refresh_token = tokens
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string());

        Ok(AuthResponse::Success(AuthSuccess::new(
            tokens.access_token,
            tokens.id_token,
            Some(refresh_token),
        )))
    }

    pub async fn current_user(&self, access_token: &str) -> ServiceResult<CurrentUser> {
        if let Some(user) = self.user_cache.get(access_token).await {
            return Ok(user);
        }

        let domain = &self.config.domain;

        let response = self
            .http_client
            .get(format!("{domain}/oauth2/userInfo"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                SharedError::ServerError
            })?;

        if !response.status().is_success() {
            return Err(SharedError::HttpMessage(
                401,
                "invalid or expired access token".into(),
            ));
        }

        let user = response.json::<CurrentUser>().await.map_err(|e| {
            println!("{:?}", e);
            SharedError::ServerError
        })?;

        self.user_cache
            .insert(access_token.to_string(), user.clone())
            .await;

        Ok(user)
    }

    fn generate_pkce() -> (String, String) {
        let mut verifier_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut verifier_bytes);

        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
        let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));

        (code_verifier, code_challenge)
    }
}
