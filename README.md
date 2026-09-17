# auth

Library crate for signing users in through Amazon Cognito's hosted/managed login (the
Authorization Code + PKCE flow), and for exchanging the resulting code for tokens. It
provides the service logic, ready-made `axum` routes (`auth::app`), and a couple of
building blocks (cookie helpers, shared `axum` state) if you'd rather wire
`AuthService` into your own controllers instead.

## What it does

- **`login_url`**: generates a PKCE `code_verifier`/`code_challenge` pair and builds the
  URL to Cognito's hosted login page.
- **`exchange_code`**: exchanges an authorization code (plus the `code_verifier` from
  `login_url`) for tokens against Cognito's token endpoint.
- **`logout_url`**: builds the URL to Cognito's hosted logout page.

None of these make any assumption about how `redirect_uri`/the PKCE verifier travel
between requests (query param, cookie, session, ...) — that's the caller's job.

## Wiring it into an app

```rust
use auth::{Config, auth::AuthService, state::ServiceState};
use std::sync::Arc;

let config = Config::new(
    Config::builder()
        .client_id("cognito-app-client-id")
        .client_secret("cognito-app-client-secret")
        // The full Cognito hosted-UI domain, not just a prefix.
        .domain("https://your-domain.auth.us-east-1.amazoncognito.com"),
)?;

let state = Arc::new(ServiceState {
    auth_service: AuthService::new(Arc::new(config)),
});

// `auth::app` returns a `utoipa_axum::router::OpenApiRouter`, so its OpenAPI paths/schemas
// can be merged into the rest of the app's spec before splitting into a plain `axum::Router`.
use utoipa_axum::router::OpenApiRouter;

let (router, openapi) = OpenApiRouter::new()
    .nest("/auth", auth::app(state))
    .split_for_parts();

let app: axum::Router = router; // serve `openapi` (e.g. via utoipa-swagger-ui) however you like
```

If you don't care about the OpenAPI spec, `OpenApiRouter<S>` also converts directly into
`axum::Router<S>` via `Into`, so `axum::Router::new().nest("/auth", auth::app(state).into())`
works too.

## Routes

Mounted under whatever prefix the parent app nests `auth::app(state)` at:

| Method | Path        | Query/Body                                          | Response                                        |
| ------ | ----------- | ---------------------------------------------------- | ------------------------------------------------ |
| GET    | `/login`    | query `redirect_uri`                                 | `LoginResponse` `{ url, code_verifier }`         |
| POST   | `/callback` | JSON `CallbackRequest` `{ code, code_verifier, redirect_uri }` | `AuthResponse` (tokens, a challenge, or `null`); sets/updates the token cookies |
| GET    | `/logout`   | query `redirect_uri`                                 | `LogoutResponse` `{ url }`; clears the token cookies |

Each handler carries a `#[utoipa::path]` annotation, so the routes and their request/response
schemas show up in the `utoipa::openapi::OpenApi` returned alongside the router.

Prefer wiring `AuthService` into your own handlers instead of using `auth::app`? A minimal
pair built directly on the service:

```rust
use auth::state::ServiceStateType;
use axum::extract::State;
use axum::response::IntoResponse;

async fn login(State(state): ServiceStateType, redirect_uri: String) -> impl IntoResponse {
    state.auth_service.login_url(&redirect_uri)
    // -> { url, code_verifier }: send the browser to `url`, and hang on to
    //    `code_verifier` (e.g. a cookie) to supply back to `callback`.
}

async fn callback(
    State(state): ServiceStateType,
    code: String,
    code_verifier: String,
    redirect_uri: String,
) -> impl IntoResponse {
    let result = state.auth_service.exchange_code(&code, &code_verifier, &redirect_uri).await;
    let jar = result.as_ref().map(auth::cookies::jar_for_response).unwrap_or_default();
    (jar, /* turn `result` into your response */)
}
```

## Cookies

`auth::cookies` has small helpers for the three token cookies (`access_token`,
`id_token`, `refresh_token`), all `httpOnly` + `secure`:

- `jar_for_response(&AuthResponse) -> CookieJar` — builds the jar for a successful
  `exchange_code` result (empty jar for anything else).
- `expired_jar() -> CookieJar` — clears all three, for a logout endpoint.
- `auth_cookie`/`expired_auth_cookie` — the lower-level builders those two use, if you
  need to set/clear one cookie individually.

## Config

Built via `Config::builder()...` passed to `Config::new()`. All three fields are
required — `Config::new` returns an error (`invalid client id` / `invalid client secret`
/ `invalid cognito domain`) if any are missing:

| Method             | Description                                                                 |
| ------------------ | ---------------------------------------------------------------------------- |
| `.client_id()`     | The Cognito app client ID.                                                   |
| `.client_secret()` | The Cognito app client secret (used as Basic auth against the token endpoint). |
| `.domain()`        | The full Cognito hosted-UI domain, e.g. `https://your-domain.auth.us-east-1.amazoncognito.com` — not just the domain prefix. |

## Errors

Failures surface as `shared::error::ServiceError::HttpMessage`. Config validation errors are
`500`s (they indicate a misconfigured deployment, not bad user input); a failed
`exchange_code` call surfaces Cognito's `error_description` (or `error`) as a `400`.
