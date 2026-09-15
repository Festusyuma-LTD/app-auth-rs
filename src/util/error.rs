use shared::error::ServiceError as SharedError;

pub type ServiceResult<T> = Result<T, SharedError>;

pub enum ServiceError {
    InvalidClientId,
    InvalidClientSecret,
    InvalidDomain,
}

impl Into<SharedError> for ServiceError {
    fn into(self) -> SharedError {
        let (code, message) = match self {
            ServiceError::InvalidClientId => (500, "invalid client id".into()),
            ServiceError::InvalidClientSecret => (500, "invalid client secret".into()),
            ServiceError::InvalidDomain => (500, "invalid cognito domain".into()),
        };

        SharedError::HttpMessage(code, message)
    }
}

impl<T> Into<ServiceResult<T>> for ServiceError {
    fn into(self) -> ServiceResult<T> {
        Err(self.into())
    }
}
