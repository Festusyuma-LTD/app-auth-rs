use crate::controllers;
use crate::util::state::ServiceState;

use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub fn app<S>(state: Arc<ServiceState>) -> OpenApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    OpenApiRouter::new()
        .routes(routes!(controllers::auth::login))
        .routes(routes!(controllers::auth::callback))
        .routes(routes!(controllers::auth::logout))
        .with_state(state)
}
