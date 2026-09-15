use crate::services::auth::AuthService;

use axum::extract::State;
use std::sync::Arc;

pub struct ServiceState {
    pub auth_service: AuthService,
}

pub type ServiceStateType = State<Arc<ServiceState>>;
