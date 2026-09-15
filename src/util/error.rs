use shared::error::AppError;

pub type ServiceResult<T> = Result<T, AppError>;

pub enum ServiceError {
    InvalidClientId,
    InvalidClientSecret,
    InvalidDomain,
}

impl Into<AppError> for ServiceError {
    fn into(self) -> AppError {
        let (code, message) = match self {
            ServiceError::InvalidClientId => (500, "invalid client id".into()),
            ServiceError::InvalidClientSecret => (500, "invalid client secret".into()),
            ServiceError::InvalidDomain => (500, "invalid cognito domain".into()),
        };

        AppError::HttpMessage(code, message)
    }
}

impl<T> Into<ServiceResult<T>> for ServiceError {
    fn into(self) -> ServiceResult<T> {
        Err(self.into())
    }
}
