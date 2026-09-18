use crate::dto::auth::AuthResponse;

use axum_extra::extract::cookie::{Cookie, CookieJar};

pub const ACCESS_TOKEN_COOKIE: &str = "access_token";
pub const ID_TOKEN_COOKIE: &str = "id_token";
pub const REFRESH_TOKEN_COOKIE: &str = "refresh_token";

pub(crate) const REFRESH_TOKEN_ROUTE: &str = "/refresh";

pub fn auth_cookie(name: &'static str, value: String, path: &str, max_age: u32) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(true)
        .path(path.to_owned())
        .max_age(time::Duration::seconds(max_age as i64))
        .build()
}

pub fn expired_auth_cookie(name: &'static str, path: &str) -> Cookie<'static> {
    Cookie::build((name, ""))
        .http_only(true)
        .secure(true)
        .path(path.to_owned())
        .max_age(time::Duration::ZERO)
        .build()
}

pub fn jar_for_response(
    response: &AuthResponse,
    refresh_token_path: &str,
    token_expiration: u32,
    refresh_expiration: u32,
) -> CookieJar {
    let mut jar = CookieJar::new();

    if let AuthResponse::Success(success) = response {
        jar = jar.add(auth_cookie(
            ACCESS_TOKEN_COOKIE,
            success.access_token.clone(),
            "/",
            token_expiration,
        ));

        if let Some(id_token) = &success.id_token {
            jar = jar.add(auth_cookie(
                ID_TOKEN_COOKIE,
                id_token.clone(),
                "/",
                token_expiration,
            ));
        }

        if let Some(refresh_token) = &success.refresh_token {
            jar = jar.add(auth_cookie(
                REFRESH_TOKEN_COOKIE,
                refresh_token.clone(),
                refresh_token_path,
                refresh_expiration,
            ));
        }
    }

    jar
}

pub fn expired_jar(refresh_token_path: &str) -> CookieJar {
    CookieJar::new()
        .add(expired_auth_cookie(ACCESS_TOKEN_COOKIE, "/"))
        .add(expired_auth_cookie(ID_TOKEN_COOKIE, "/"))
        .add(expired_auth_cookie(
            REFRESH_TOKEN_COOKIE,
            refresh_token_path,
        ))
}
