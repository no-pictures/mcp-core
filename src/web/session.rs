// SPDX-FileCopyrightText: 2025-2026 Stefan Grönke <stefan@gronke.net>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Web login: a SameSite=Strict signed session cookie set by `POST /login` after a
//! constant-time token check. The gating middleware accepts the cookie OR an `Authorization`
//! Bearer/Basic header, so programmatic MCP clients authenticate without a cookie.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{FromRef, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::{Cookie, Key, SameSite, SignedCookieJar};
use subtle::ConstantTimeEq;

const COOKIE_NAME: &str = "mcp_session";

/// Shared web-auth state: the optional token plus the per-process cookie signing key.
///
/// `token` `None` leaves the app open (no login required). The signing key is random per
/// process, so a restart invalidates existing sessions (clients re-login).
#[derive(Clone)]
pub struct WebAuth {
    token: Option<Arc<str>>,
    key: Key,
}

impl WebAuth {
    /// Build web-auth from the optional configured token.
    pub fn new(token: Option<&str>) -> Self {
        Self {
            token: token.map(Arc::from),
            key: Key::generate(),
        }
    }

    /// Whether a token is configured (i.e. login is required).
    pub fn auth_required(&self) -> bool {
        self.token.is_some()
    }
}

impl FromRef<WebAuth> for Key {
    fn from_ref(auth: &WebAuth) -> Self {
        auth.key.clone()
    }
}

/// Constant-time token comparison (avoids leaking the token via timing).
fn token_eq(presented: &str, token: &str) -> bool {
    presented.as_bytes().ct_eq(token.as_bytes()).into()
}

/// Whether `headers` carries a valid `Authorization: Bearer <token>` or Basic `<user:token>`.
fn header_authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    if let Some(bearer) = value.strip_prefix("Bearer ") {
        return token_eq(bearer, token);
    }
    if let Some(basic) = value.strip_prefix("Basic ") {
        if let Some(decoded) = decode_basic(basic) {
            if let Some((_user, pass)) = decoded.split_once(':') {
                return token_eq(pass, token);
            }
        }
    }
    false
}

fn decode_basic(input: &str) -> Option<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input)
        .ok()?;
    String::from_utf8(bytes).ok()
}

/// A top-level browser navigation (gets a redirect to the login page on failure) vs an
/// API/XHR call (gets a 401).
fn is_navigation(headers: &HeaderMap) -> bool {
    if let Some(mode) = headers.get("sec-fetch-mode").and_then(|v| v.to_str().ok()) {
        return mode == "navigate";
    }
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|a| a.contains("text/html"))
}

/// `POST /login` body (`application/x-www-form-urlencoded`).
#[derive(serde::Deserialize)]
pub struct LoginForm {
    token: String,
}

/// `POST /login`: set the session cookie when the token matches; otherwise `401`.
pub async fn login(
    State(auth): State<WebAuth>,
    jar: SignedCookieJar,
    Form(form): Form<LoginForm>,
) -> Response {
    let Some(token) = auth.token.as_deref() else {
        return Redirect::to("/ui/").into_response();
    };
    if token_eq(&form.token, token) {
        let cookie = Cookie::build((COOKIE_NAME, "ok"))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .path("/")
            .build();
        (jar.add(cookie), Redirect::to("/ui/")).into_response()
    } else {
        (StatusCode::UNAUTHORIZED, "invalid token").into_response()
    }
}

/// `POST /logout`: clear the session cookie.
pub async fn logout(jar: SignedCookieJar) -> Response {
    let removal = Cookie::build(COOKIE_NAME).path("/").build();
    (jar.remove(removal), Redirect::to("/")).into_response()
}

/// Gating middleware: allow when no token is configured, when a valid session cookie is
/// present, or when a valid `Authorization` header is present. On failure, redirect browser
/// navigations to `/` (the login page) and return `401` to API/MCP callers.
pub async fn require_auth(
    State(auth): State<WebAuth>,
    jar: SignedCookieJar,
    req: Request<Body>,
    next: Next,
) -> Response {
    let Some(token) = auth.token.as_deref() else {
        return next.run(req).await;
    };
    if jar.get(COOKIE_NAME).is_some() || header_authorized(req.headers(), token) {
        return next.run(req).await;
    }
    if is_navigation(req.headers()) {
        Redirect::to("/").into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::Request,
        middleware::from_fn_with_state,
        routing::{get, post},
        Router,
    };
    use tower::util::ServiceExt;

    /// The `app_router` wiring in miniature: a gated route plus the public login/logout pair,
    /// all sharing one `WebAuth` (same signing key).
    fn app(token: Option<&str>) -> Router {
        let auth = WebAuth::new(token);
        Router::new()
            .route("/protected", get(|| async { "SECRET" }))
            .layer(from_fn_with_state(auth.clone(), require_auth))
            .route("/login", post(login))
            .route("/logout", post(logout))
            .with_state(auth)
    }

    async fn request(app: &Router, req: Request<Body>) -> Response {
        app.clone().oneshot(req).await.unwrap()
    }

    async fn get_protected(app: &Router, headers: &[(&str, &str)]) -> Response {
        let mut builder = Request::builder().uri("/protected");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        request(app, builder.body(Body::empty()).unwrap()).await
    }

    async fn post_login(app: &Router, token: &str) -> Response {
        let req = Request::builder()
            .method("POST")
            .uri("/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("token={token}")))
            .unwrap();
        request(app, req).await
    }

    /// The `NAME=value` pair of a response's session cookie, for replaying in a `Cookie` header.
    fn session_cookie(res: &Response) -> String {
        let set_cookie = res.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(set_cookie.starts_with(COOKIE_NAME), "cookie: {set_cookie}");
        set_cookie.split(';').next().unwrap().to_string()
    }

    #[tokio::test]
    async fn no_token_leaves_everything_open() {
        let res = get_protected(&app(None), &[]).await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_call_without_credentials_is_401() {
        let res = get_protected(&app(Some("sekret")), &[]).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn navigation_without_credentials_redirects_to_login() {
        let app = app(Some("sekret"));
        for headers in [
            &[("sec-fetch-mode", "navigate")][..],
            &[("accept", "text/html,application/xhtml+xml")][..],
        ] {
            let res = get_protected(&app, headers).await;
            assert_eq!(res.status(), StatusCode::SEE_OTHER);
            assert_eq!(res.headers()[header::LOCATION], "/");
        }
    }

    #[tokio::test]
    async fn fetch_metadata_beats_the_accept_heuristic() {
        // An XHR that happens to accept text/html is still an API call, not a navigation.
        let res = get_protected(
            &app(Some("sekret")),
            &[("sec-fetch-mode", "cors"), ("accept", "text/html")],
        )
        .await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn bearer_and_basic_headers_authorize() {
        use base64::Engine;
        let app = app(Some("sekret"));

        let res = get_protected(&app, &[("authorization", "Bearer sekret")]).await;
        assert_eq!(res.status(), StatusCode::OK);

        let creds = base64::engine::general_purpose::STANDARD.encode("anyuser:sekret");
        let basic = format!("Basic {creds}");
        let res = get_protected(&app, &[("authorization", basic.as_str())]).await;
        assert_eq!(res.status(), StatusCode::OK);

        let res = get_protected(&app, &[("authorization", "Bearer wrong")]).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn login_sets_a_hardened_cookie_that_authorizes() {
        let app = app(Some("sekret"));
        let res = post_login(&app, "sekret").await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(res.headers()[header::LOCATION], "/ui/");

        let set_cookie = res.headers()[header::SET_COOKIE].to_str().unwrap();
        for attribute in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
            assert!(set_cookie.contains(attribute), "cookie: {set_cookie}");
        }

        let cookie = session_cookie(&res);
        let res = get_protected(&app, &[("cookie", cookie.as_str())]).await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn login_with_wrong_token_is_401() {
        let res = post_login(&app(Some("sekret")), "wrong").await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert!(res.headers().get(header::SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn forged_cookie_is_rejected() {
        // A cookie not signed with this process's key does not authorize.
        let res = get_protected(
            &app(Some("sekret")),
            &[("cookie", "mcp_session=ok"), ("sec-fetch-mode", "cors")],
        )
        .await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn logout_expires_the_cookie_and_redirects_home() {
        let app = app(Some("sekret"));
        // The removal Set-Cookie is only emitted for a cookie the request carried, so sign in
        // first and send the session cookie with the logout.
        let cookie = session_cookie(&post_login(&app, "sekret").await);

        let res = request(
            &app,
            Request::builder()
                .method("POST")
                .uri("/logout")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(res.headers()[header::LOCATION], "/");
        let set_cookie = res.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(set_cookie.starts_with(COOKIE_NAME), "cookie: {set_cookie}");
        assert!(set_cookie.contains("Max-Age=0"), "cookie: {set_cookie}");
    }
}
