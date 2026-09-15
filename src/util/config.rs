use crate::util::error::{ServiceError, ServiceResult};

use shared::error::AppError;

pub struct Config {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) domain: String,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    pub fn new(value: ConfigBuilder) -> ServiceResult<Self> {
        let client_id = value
            .client_id
            .ok_or_else(|| -> AppError { ServiceError::InvalidClientId.into() })?;

        let client_secret = value
            .client_secret
            .ok_or_else(|| -> AppError { ServiceError::InvalidClientSecret.into() })?;

        let domain = value
            .domain
            .ok_or_else(|| -> AppError { ServiceError::InvalidDomain.into() })?;

        Ok(Self {
            client_id,
            client_secret,
            domain,
        })
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    client_id: Option<String>,
    client_secret: Option<String>,
    domain: Option<String>,
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
}
