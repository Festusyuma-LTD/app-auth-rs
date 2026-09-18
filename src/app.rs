use crate::controllers;
use crate::util::middleware as auth_middleware;
use crate::util::state::ServiceState;

use axum::middleware;
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub fn app<S>(state: Arc<ServiceState>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    let verify = OpenApiRouter::new()
        .routes(routes!(controllers::auth::verify))
        .route_layer(middleware::from_fn(auth_middleware::require_user))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware::populate_user,
        ));

    OpenApiRouter::new()
        .routes(routes!(controllers::auth::login))
        .routes(routes!(controllers::auth::callback))
        .routes(routes!(controllers::auth::refresh))
        .routes(routes!(controllers::auth::logout))
        .merge(verify)
        .with_state(state)
}
