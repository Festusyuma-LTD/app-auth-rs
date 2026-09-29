use crate::dto::auth::CurrentUser;
use shared::error::ServiceError;
use shared::response::ServiceResult;

pub fn get_user_email(user: &CurrentUser) -> ServiceResult<String> {
    user.attributes
        .get("email")
        .and_then(|e| e.as_str())
        .map(|e| e.to_string())
        .ok_or_else(|| ServiceError::Http(403))
}
