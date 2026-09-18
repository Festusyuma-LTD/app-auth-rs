use crate::dto::auth::CurrentUser;
use crate::util::cookies;
use crate::util::state::ServiceState;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::CookieJar;
use shared::error::ServiceError;
use std::sync::Arc;

/// Looks for an `access_token` cookie and, if present and valid, resolves it to a
/// [`CurrentUser`] (via `AuthService::current_user`) and inserts it into the request's
/// extensions for downstream handlers/middleware to read via `Extension<CurrentUser>`.
///
/// Never rejects the request — a missing, invalid, or expired token just means no
/// `CurrentUser` gets inserted. Pair with [`require_user`] (after this in the layer
/// stack) to actually enforce authentication on a route.
pub async fn populate_user(
    State(state): State<Arc<ServiceState>>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(access_token) = jar.get(cookies::ACCESS_TOKEN_COOKIE) {
        if let Ok(user) = state.auth_service.current_user(access_token.value()).await {
            req.extensions_mut().insert(user);
        }
    }

    next.run(req).await
}

/// Rejects the request with `401` unless a [`CurrentUser`] is already present in its
/// extensions. Must run after [`populate_user`] in the layer stack — on its own it can
/// only ever see an empty request and will always reject.
pub async fn require_user(req: Request, next: Next) -> Response {
    if req.extensions().get::<CurrentUser>().is_none() {
        return ServiceError::HttpMessage(401, "not logged in".into()).into_response();
    }

    next.run(req).await
}
