use crate::controllers;
use crate::util::state::ServiceState;

use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

pub fn app<S>(state: Arc<ServiceState>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/login", get(controllers::auth::login))
        .route("/callback", post(controllers::auth::callback))
        .route("/logout", get(controllers::auth::logout))
        .with_state(state)
}
