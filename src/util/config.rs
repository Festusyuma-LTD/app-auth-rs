use crate::util::error::{ServiceError, ServiceResult};

use shared::error::ServiceError as SharedError;

const DEFAULT_TOKEN_EXPIRATION: u32 = 1800;
const DEFAULT_REFRESH_EXPIRATION: u32 = 86400;

pub struct Config {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) domain: String,
    pub(crate) base_path: String,
    pub(crate) token_expiration: u32,
    pub(crate) refresh_expiration: u32,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    pub fn new(value: ConfigBuilder) -> ServiceResult<Self> {
        let client_id = value
            .client_id
            .ok_or_else(|| -> SharedError { ServiceError::InvalidClientId.into() })?;

        let client_secret = value
            .client_secret
            .ok_or_else(|| -> SharedError { ServiceError::InvalidClientSecret.into() })?;

        let domain = value
            .domain
            .ok_or_else(|| -> SharedError { ServiceError::InvalidDomain.into() })?;

        Ok(Self {
            client_id,
            client_secret,
            domain,
            base_path: value.base_path.unwrap_or_default(),
            token_expiration: value.token_expiration.unwrap_or(DEFAULT_TOKEN_EXPIRATION),
            refresh_expiration: value
                .refresh_expiration
                .unwrap_or(DEFAULT_REFRESH_EXPIRATION),
        })
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    client_id: Option<String>,
    client_secret: Option<String>,
    domain: Option<String>,
    base_path: Option<String>,
    token_expiration: Option<u32>,
    refresh_expiration: Option<u32>,
}

impl ConfigBuilder {
    pub fn client_id(mut self, client_id: &str) -> Self {
        self.client_id = Some(client_id.to_string());
        self
    }

    pub fn client_secret(mut self, client_secret: &str) -> Self {
        self.client_secret = Some(client_secret.to_string());
        self
    }

    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_string());
        self
    }

    pub fn base_path(mut self, base_path: &str) -> Self {
        self.base_path = Some(base_path.to_string());
        self
    }

    pub fn token_expiration(mut self, seconds: Option<u32>) -> Self {
        self.token_expiration = seconds;
        self
    }

    pub fn refresh_expiration(mut self, seconds: Option<u32>) -> Self {
        self.refresh_expiration = seconds;
        self
    }
}
