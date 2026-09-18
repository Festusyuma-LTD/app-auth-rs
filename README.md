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
- **`refresh_token`**: exchanges a refresh token for a new access/ID token pair against
  Cognito's token endpoint.
- **`current_user`**: resolves an access token to the Cognito user it belongs to, via
  `/oauth2/userInfo`. Results are cached per access token for 30 seconds.
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
        .domain("https://your-domain.auth.us-east-1.amazoncognito.com")
        // Where this service is reachable from the browser: gateway base path (if any)
        // + wherever you nest `auth::app` below. Omit if mounted at the root.
        .base_path("/api/auth")
        // `Max-Age`, in seconds, for the access/ID and refresh token cookies. Optional —
        // default to 1800 (30 minutes) and 86400 (24 hours) respectively.
        .token_expiration(3600)
        .refresh_expiration(2_592_000),
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
| POST   | `/refresh`  | `refresh_token` cookie (set by `/callback`)          | `AuthResponse` with fresh tokens; updates the token cookies. `401` if the cookie is missing |
| GET    | `/verify`   | `access_token` cookie (set by `/callback`/`/refresh`) | `CurrentUser`. `401` if the cookie is missing/invalid/expired |
| GET    | `/logout`   | query `redirect_uri`                                 | `LogoutResponse` `{ url }`; clears the token cookies |

`/verify` is `populate_user` + `require_user` (see [Middleware](#middleware)) attached to a
single route — it's the simplest way to check "is this access token still good" and get the
user back in one call, scoped so those two middlewares don't affect the other routes above.

Each handler carries a `#[utoipa::path]` annotation, so the routes and their request/response
schemas show up in the `utoipa::openapi::OpenApi` returned alongside the router.

`/callback`, `/refresh`, and `/logout` all scope the `refresh_token` cookie's `Path` to the
refresh endpoint using `Config::base_path` (see [Cookies](#cookies) below) instead of the
whole app.

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
    let jar = result
        .as_ref()
        .map(|r| state.auth_service.jar_for_response(r))
        .unwrap_or_default();
    (jar, /* turn `result` into your response */)
}
```

## Cookies

`AuthService` builds the token cookies for you, all `httpOnly` + `secure`.
`access_token`/`id_token` are scoped to `Path=/` with `Max-Age=Config::token_expiration`
(defaults to `1800`, 30 minutes); `refresh_token` is scoped to just the refresh endpoint
(`AuthService::refresh_token_path()`, i.e. `Config::base_path` + `/refresh`) with
`Max-Age=Config::refresh_expiration` (defaults to `86400`, 24 hours), so it's never sent on
any other request.

- `AuthService::jar_for_response(&AuthResponse) -> CookieJar` — builds the jar for a
  successful `exchange_code`/`refresh_token` result (empty jar for anything else).
- `AuthService::expired_jar() -> CookieJar` — clears all three, for a logout endpoint.
- `AuthService::refresh_token_path() -> String` — computes `{Config::base_path}/refresh`.

`auth::app`'s handlers already call these for you. The lower-level free functions in
`auth::cookies` (`jar_for_response`, `expired_jar`, `auth_cookie`, `expired_auth_cookie`)
are also public if you need to build cookies outside of `AuthService` — but note
`jar_for_response`/`expired_jar` there take the refresh path and `Max-Age`s as explicit
arguments, so **always source them from the same `AuthService`** (via
`refresh_token_path()`); cookies with a mismatched `Path` are distinct as far as the
browser is concerned, so a mismatch silently fails to clear a previously-set
`refresh_token` cookie on logout.

## Middleware

`auth::middleware` has two `axum` middlewares, meant to be layered together on any router
that needs to know who's calling:

- **`populate_user`** — reads the `access_token` cookie, resolves it to a `CurrentUser`
  via `AuthService::current_user` (Cognito's `/oauth2/userInfo`, cached for 30s per
  token), and inserts it into the request's extensions. Never rejects the request — a
  missing, invalid, or expired token just means no `CurrentUser` gets inserted, so it's
  safe to apply globally even to routes that don't require auth.
- **`require_user`** — rejects with `401` unless a `CurrentUser` is already in the
  request's extensions. Must run *after* `populate_user` in the layer stack (`axum`
  applies the last-added `.layer()` outermost, so add `populate_user` last); on its own,
  with nothing upstream setting a `CurrentUser`, it always rejects.

Handlers read the resolved user with the usual `axum::Extension<auth::dto::auth::CurrentUser>`
extractor. `CurrentUser` always has `sub`/`username`; anything else Cognito returns
(`email`, `email_verified`, custom attributes, ...) is in `attributes: HashMap<String,
serde_json::Value>`, since that depends on the access token's scopes.

```rust
use auth::dto::auth::CurrentUser;
use axum::{Extension, Router, middleware};

let protected = Router::new()
    .route("/me", axum::routing::get(|Extension(user): Extension<CurrentUser>| async move {
        user.sub
    }))
    .layer(middleware::from_fn(auth::middleware::require_user));

let app = Router::new()
    .merge(protected) // 401s without a valid access_token cookie
    .route("/public", axum::routing::get(|| async { "ok" })) // works either way
    .layer(middleware::from_fn_with_state(state.clone(), auth::middleware::populate_user))
    .with_state(state);
```

## Config

Built via `Config::builder()...` passed to `Config::new()`. `client_id`/`client_secret`/
`domain` are required — `Config::new` returns an error (`invalid client id` / `invalid
client secret` / `invalid cognito domain`) if any are missing. The rest are optional:

| Method                 | Required | Description                                                                 |
| ---------------------- | -------- | ---------------------------------------------------------------------------- |
| `.client_id()`         | yes      | The Cognito app client ID.                                                   |
| `.client_secret()`     | yes      | The Cognito app client secret (used as Basic auth against the token endpoint). |
| `.domain()`            | yes      | The full Cognito hosted-UI domain, e.g. `https://your-domain.auth.us-east-1.amazoncognito.com` — not just the domain prefix. |
| `.base_path()`         | no       | Where this service is reachable from the browser's point of view: the gateway's base path (if any) plus wherever the consuming app nests `auth::app` (e.g. `"/api/auth"`). Defaults to `""` (mounted at the root). Used only to scope the `refresh_token` cookie's `Path`. |
| `.token_expiration()`  | no       | `Max-Age`, in seconds, for the `access_token`/`id_token` cookies. Defaults to `1800` (30 minutes). |
| `.refresh_expiration()`| no       | `Max-Age`, in seconds, for the `refresh_token` cookie. Defaults to `86400` (24 hours). |

## Errors

Failures surface as `shared::error::ServiceError::HttpMessage`. Config validation errors are
`500`s (they indicate a misconfigured deployment, not bad user input); a failed
`exchange_code`/`refresh_token` call surfaces Cognito's `error_description` (or `error`) as a
`400`.
