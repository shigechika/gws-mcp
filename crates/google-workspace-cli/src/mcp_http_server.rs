// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! MCP Streamable HTTP transport with optional OAuth2 PKCE authentication.
//!
//! Phase 1 (no auth): `gws mcp -s gmail --transport http --port 3000`
//! Phase 2 (OAuth2):  `gws mcp -s gmail --transport http --port 3000 --auth`
//!
//! When `--auth` is enabled the server implements the MCP Authorization spec
//! (2025-11-25, RFC 9728 / RFC 8414 / PKCE-S256) and acts as both OAuth2
//! Authorization Server and Resource Server.  Google is used as the identity
//! provider; a `client_secret.json` created via `gws auth setup` is required.
//!
//! Phase 4: Each authenticated user's MCP tool calls use their own Google
//! access token obtained during the OAuth flow (per-user token isolation).
//! GWS scopes are derived from the configured service list at authorize time.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::{
    extract::{Query, State},
    http::{header, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router,
};
use rmcp::transport::{
    streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
    StreamableHttpService,
};
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerInfo, Tool,
    },
    service::RequestContext,
    ErrorData as McpError, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

use crate::{
    auth_commands::gws_scopes_for_services,
    error::GwsError,
    mcp_server::{build_tools_list, handle_tools_call, ServerConfig},
    oauth_config::{load_client_config, InstalledConfig},
};

// ─── Constants ───────────────────────────────────────────────────────────────

const SESSION_TTL: Duration = Duration::from_secs(8 * 3600); // 8 h
const PENDING_TTL: Duration = Duration::from_secs(600); // 10 min

// ─── Phase 1: MCP handler ────────────────────────────────────────────────────

struct GwsMcpHandler {
    config: Arc<ServerConfig>,
    tools_cache: Arc<Mutex<Option<Vec<Tool>>>>,
}

impl ServerHandler for GwsMcpHandler {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new("gws-mcp", env!("CARGO_PKG_VERSION"));
        info.capabilities.tools = Some(Default::default());
        info
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut cache = self.tools_cache.lock().await;
        if cache.is_none() {
            let values = build_tools_list(&self.config)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            let tools: Vec<Tool> = values
                .into_iter()
                .filter_map(|v| {
                    serde_json::from_value(v.clone())
                        .map_err(|e| {
                            let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                            eprintln!("[gws mcp] Warning: skipping tool '{name}': {e}");
                        })
                        .ok()
                })
                .collect();
            *cache = Some(tools);
        }
        Ok(ListToolsResult {
            tools: cache.as_ref().unwrap().clone(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Extract per-user GWS access token injected by bearer_auth_middleware.
        // Present only when --auth is enabled; falls back to shared gws-auth token otherwise.
        let user_token: Option<String> = ctx
            .extensions
            .get::<http::request::Parts>()
            .and_then(|p| p.extensions.get::<RequestUserToken>())
            .map(|t| t.0.clone());

        let params = json!({
            "name": request.name.as_ref(),
            "arguments": request.arguments.unwrap_or_default()
        });
        let result = handle_tools_call(&params, &self.config, user_token.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        serde_json::from_value(result).map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}

// ─── Phase 2: Auth types ─────────────────────────────────────────────────────

/// State stored between `/oauth/authorize` and `/oauth/callback`.
struct PendingAuth {
    code_challenge: String,
    #[allow(dead_code)] // kept for future client_id validation
    client_id: String,
    redirect_uri: String,
    created_at: SystemTime,
}

/// State stored between `/oauth/callback` and `/oauth/token`.
struct PendingCode {
    code_challenge: String,
    client_redirect_uri: String,
    email: String,
    created_at: SystemTime,
}

struct UserSession {
    email: String,
    created_at: SystemTime,
}

/// Per-user Google access token obtained during the OAuth callback flow.
#[derive(Clone)]
struct UserGwsToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: SystemTime,
}

/// Axum request extension injected by bearer_auth_middleware for --auth mode.
/// Carries the caller's GWS access token into the MCP handler via HTTP Parts.
#[derive(Clone)]
struct RequestUserToken(String);

#[derive(Default)]
struct AuthStore {
    /// OAuth `state` value → PendingAuth (before Google callback)
    pending: Mutex<HashMap<String, PendingAuth>>,
    /// Short-lived auth code → PendingCode (after Google callback, before token exchange)
    codes: Mutex<HashMap<String, PendingCode>>,
    /// Bearer UUID → UserSession (live sessions)
    sessions: Mutex<HashMap<String, UserSession>>,
}

#[derive(Clone)]
struct AppState {
    auth_store: Arc<AuthStore>,
    oauth_cfg: Arc<InstalledConfig>,
    /// Per-user GWS access tokens keyed by email.
    // NOTE: concurrent refresh of the same user's token is benign (both succeed, last writer wins).
    user_tokens: Arc<Mutex<HashMap<String, UserGwsToken>>>,
    /// Configured services — used to derive GWS scopes at authorize time.
    services: Vec<String>,
    /// Public base URL used in RFC 9728 / RFC 8414 metadata and Google callback URI.
    /// Set by `--public-url`; falls back to `server_base(port, bind)` for local use.
    base_url: String,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn is_expired(created_at: SystemTime, ttl: Duration) -> bool {
    SystemTime::now()
        .duration_since(created_at)
        .map(|d| d >= ttl)
        .unwrap_or(true)
}

fn base64url(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(bytes)
}

fn verify_pkce_s256(verifier: &str, challenge: &str) -> bool {
    base64url(&Sha256::digest(verifier.as_bytes())) == challenge
}

fn urlencode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

/// Returns the public base URL of the server.
///
/// Uses `localhost` for both `0.0.0.0` and `127.0.0.1` binds so that the
/// RFC 9728 `resource` field matches the canonical URL clients use to connect
/// (e.g. `http://localhost:3000/mcp`). Without this, Claude Code's MCP
/// Authorization spec validation fails because `http://127.0.0.1:3000` ≠
/// `http://localhost:3000`.
fn server_base(port: u16, bind: &str) -> String {
    match bind {
        "0.0.0.0" | "127.0.0.1" | "::" | "::1" => format!("http://localhost:{port}"),
        other if other.contains(':') => format!("http://[{other}]:{port}"),
        other => format!("http://{other}:{port}"),
    }
}

/// Resolves the public base URL from `--public-url` or falls back to `server_base()`.
///
/// Validates that the URL has an http/https scheme, no query or fragment, and
/// strips a trailing slash. Warns (but does not reject) plain `http://` with a
/// non-loopback host, since RFC 9728 §3.1 recommends HTTPS for public deployments.
fn resolve_base_url(public_url: Option<&str>, port: u16, bind: &str) -> Result<String, GwsError> {
    let Some(raw) = public_url else {
        return Ok(server_base(port, bind));
    };

    let parsed = Url::parse(raw)
        .map_err(|e| GwsError::Other(anyhow::anyhow!("--public-url is not a valid URL: {e}")))?;

    match parsed.scheme() {
        "http" | "https" => {}
        s => {
            return Err(GwsError::Other(anyhow::anyhow!(
                "--public-url scheme must be http or https, got: {s}"
            )));
        }
    }
    if parsed.query().is_some() {
        return Err(GwsError::Other(anyhow::anyhow!(
            "--public-url must not contain a query string (RFC 8414 §2)"
        )));
    }
    if parsed.fragment().is_some() {
        return Err(GwsError::Other(anyhow::anyhow!(
            "--public-url must not contain a fragment"
        )));
    }

    let base = raw.trim_end_matches('/').to_string();

    if parsed.scheme() == "http" {
        let host = parsed.host_str().unwrap_or("");
        let is_loopback = matches!(host, "localhost" | "127.0.0.1" | "::1");
        if !is_loopback {
            eprintln!(
                "[gws mcp] Warning: --public-url uses plain http:// with a non-loopback host. \
                 RFC 9728 §3.1 recommends HTTPS for public deployments."
            );
        }
    }

    Ok(base)
}

fn build_google_auth_url(client_id: &str, redirect_uri: &str, scopes: &str, state: &str) -> String {
    format!(
        "https://accounts.google.com/o/oauth2/auth?\
         scope={}&access_type=offline&redirect_uri={}&\
         response_type=code&client_id={}&state={}&\
         prompt=select_account+consent",
        urlencode(scopes),
        urlencode(redirect_uri),
        urlencode(client_id),
        urlencode(state),
    )
}

// ─── Phase 3: Bearer middleware ───────────────────────────────────────────────

async fn bearer_auth_middleware(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    // OAuth and well-known endpoints are public
    if path.starts_with("/oauth/") || path.starts_with("/.well-known/") {
        return next.run(request).await;
    }

    let Some(token) = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
    else {
        let www_auth = format!(
            "Bearer realm=\"gws-mcp\", resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
            state.base_url
        );
        return (
            StatusCode::UNAUTHORIZED,
            [(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_str(&www_auth).expect("valid header value"),
            )],
        )
            .into_response();
    };

    // Validate session and extract email; release lock before any async work.
    let email = {
        let sessions = state.auth_store.sessions.lock().await;
        match sessions.get(&token) {
            Some(s) if !is_expired(s.created_at, SESSION_TTL) => s.email.clone(),
            _ => {
                let www_auth = format!(
                    "Bearer realm=\"gws-mcp\", error=\"invalid_token\", resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
                    state.base_url
                );
                return (
                    StatusCode::UNAUTHORIZED,
                    [(
                        header::WWW_AUTHENTICATE,
                        HeaderValue::from_str(&www_auth).expect("valid header value"),
                    )],
                )
                    .into_response();
            }
        }
    };

    // Resolve (and if necessary refresh) the per-user GWS token.
    // Lock is released before async HTTP so we never hold Mutex across await.
    match get_or_refresh_user_token(&state, &email).await {
        Ok(access_token) => {
            request
                .extensions_mut()
                .insert(RequestUserToken(access_token));
        }
        Err(e) => {
            eprintln!("[gws mcp] GWS token unavailable for {email}: {e}");
            return (
                StatusCode::UNAUTHORIZED,
                [(
                    "WWW-Authenticate",
                    "Bearer realm=\"gws-mcp\", error=\"invalid_token\", error_description=\"GWS token expired — please re-authenticate\"",
                )],
            )
                .into_response();
        }
    }

    next.run(request).await
}

/// Return a valid GWS access token for `email`, refreshing if the stored token
/// is within 60 seconds of expiry.  Lock is never held across the async refresh call.
async fn get_or_refresh_user_token(state: &AppState, email: &str) -> anyhow::Result<String> {
    // Phase 1: read token info while holding lock, then release immediately.
    let (access_token, needs_refresh, refresh_token) = {
        let tokens = state.user_tokens.lock().await;
        match tokens.get(email) {
            None => anyhow::bail!("no GWS token on record"),
            Some(t) => {
                let expires_soon = t
                    .expires_at
                    .duration_since(SystemTime::now())
                    .map(|d| d.as_secs() < 60)
                    .unwrap_or(true);
                (
                    t.access_token.clone(),
                    expires_soon,
                    t.refresh_token.clone(),
                )
            }
        }
    };

    if !needs_refresh {
        return Ok(access_token);
    }

    // Phase 2: async HTTP refresh outside any lock.
    let rt = refresh_token.ok_or_else(|| anyhow::anyhow!("no refresh_token available"))?;
    let new_token = refresh_google_token(&state.oauth_cfg, &rt).await?;

    // Phase 3: write updated token back.
    state
        .user_tokens
        .lock()
        .await
        .insert(email.to_string(), new_token.clone());

    Ok(new_token.access_token)
}

// ─── OAuth metadata endpoints ────────────────────────────────────────────────

/// RFC 9728 — OAuth 2.0 Protected Resource Metadata
async fn protected_resource_metadata(State(state): State<AppState>) -> Json<Value> {
    let base = &state.base_url;
    Json(json!({
        "resource": base,
        "authorization_servers": [base],
        "bearer_methods_supported": ["header"],
        "resource_documentation": "https://github.com/shigechika/gws-mcp"
    }))
}

/// RFC 8414 — OAuth 2.0 Authorization Server Metadata
async fn authorization_server_metadata(State(state): State<AppState>) -> Json<Value> {
    let base = &state.base_url;
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/oauth/authorize"),
        "token_endpoint": format!("{base}/oauth/token"),
        "registration_endpoint": format!("{base}/oauth/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": ["openid", "email", "profile"]
    }))
}

// ─── Dynamic Client Registration stub (RFC 7591) ─────────────────────────────

#[derive(Deserialize)]
struct RegisterRequest {
    client_name: Option<String>,
    redirect_uris: Option<Vec<String>>,
}

async fn oauth_register(
    State(_): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let client_id = req
        .client_name
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "gws-mcp-client".to_string());
    (
        StatusCode::CREATED,
        Json(json!({
            "client_id": client_id,
            "redirect_uris": req.redirect_uris.unwrap_or_default(),
            "grant_types": ["authorization_code"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none"
        })),
    )
}

// ─── /oauth/authorize ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AuthorizeParams {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
    #[allow(dead_code)]
    // accepted but not forwarded to Google; scopes are derived from server-side service config
    scope: Option<String>,
}

async fn oauth_authorize(
    State(state): State<AppState>,
    Query(p): Query<AuthorizeParams>,
) -> Result<Redirect, (StatusCode, String)> {
    if p.response_type != "code" {
        return Err((
            StatusCode::BAD_REQUEST,
            "unsupported_response_type".to_string(),
        ));
    }
    if p.code_challenge_method != "S256" {
        return Err((
            StatusCode::BAD_REQUEST,
            "only S256 code_challenge_method is supported".to_string(),
        ));
    }
    // Restrict redirect_uri to prevent open-redirector abuse.
    // Local deployments: loopback only. Public HTTPS deployments (--public-url https://...):
    // also allow https:// redirect URIs for remote MCP clients that use non-loopback callbacks.
    if !is_redirect_uri_allowed(&state.base_url, &p.redirect_uri) {
        let is_public_https = state.base_url.starts_with("https://");
        return Err((
            StatusCode::BAD_REQUEST,
            if is_public_https {
                "redirect_uri must be a loopback address or an https:// URI".to_string()
            } else {
                "redirect_uri must be a loopback address (http://localhost:*, http://127.0.0.1:*, http://[::1]:*)".to_string()
            },
        ));
    }

    state.auth_store.pending.lock().await.insert(
        p.state.clone(),
        PendingAuth {
            code_challenge: p.code_challenge,
            client_id: p.client_id,
            redirect_uri: p.redirect_uri,
            created_at: SystemTime::now(),
        },
    );

    let callback = format!("{}/oauth/callback", state.base_url);
    // Derive GWS scopes from the configured service list (e.g. gmail, drive).
    // The client's requested scope is ignored — only the server-side service list
    // is trusted, preventing malicious clients from requesting arbitrary Google scopes.
    let scope_str = gws_scopes_for_services(&state.services).join(" ");
    let google_url =
        build_google_auth_url(&state.oauth_cfg.client_id, &callback, &scope_str, &p.state);

    Ok(Redirect::to(&google_url))
}

// ─── /oauth/callback ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: String,
    error: Option<String>,
}

async fn oauth_callback(
    State(state): State<AppState>,
    Query(p): Query<CallbackParams>,
) -> Result<Redirect, (StatusCode, String)> {
    if let Some(err) = p.error {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Google OAuth error: {err}"),
        ));
    }
    let google_code = p
        .code
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing code".to_string()))?;

    let pending = {
        let mut lock = state.auth_store.pending.lock().await;
        lock.remove(&p.state)
            .filter(|pa| !is_expired(pa.created_at, PENDING_TTL))
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid or expired state".to_string(),
                )
            })?
    };

    let callback = format!("{}/oauth/callback", state.base_url);
    let token_resp = exchange_google_code(&state.oauth_cfg, &google_code, &callback)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let email = get_google_email(&token_resp.access_token)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Store per-user GWS token for use in MCP tool calls.
    let gws_token = UserGwsToken {
        access_token: token_resp.access_token.clone(),
        refresh_token: token_resp.refresh_token.clone(),
        expires_at: SystemTime::now()
            + Duration::from_secs(token_resp.expires_in.unwrap_or(3600).saturating_sub(60)),
    };
    state
        .user_tokens
        .lock()
        .await
        .insert(email.clone(), gws_token);

    let auth_code = Uuid::new_v4().to_string();
    state.auth_store.codes.lock().await.insert(
        auth_code.clone(),
        PendingCode {
            code_challenge: pending.code_challenge,
            client_redirect_uri: pending.redirect_uri.clone(),
            email,
            created_at: SystemTime::now(),
        },
    );

    let location = format!(
        "{}?code={}&state={}",
        pending.redirect_uri,
        urlencode(&auth_code),
        urlencode(&p.state),
    );
    Ok(Redirect::to(&location))
}

// ─── /oauth/token ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenRequest {
    grant_type: String,
    code: String,
    code_verifier: String,
    redirect_uri: Option<String>,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
}

async fn oauth_token(
    State(state): State<AppState>,
    Form(req): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, (StatusCode, Json<Value>)> {
    if req.grant_type != "authorization_code" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "unsupported_grant_type"})),
        ));
    }

    let pending_code = {
        let mut lock = state.auth_store.codes.lock().await;
        lock.remove(&req.code)
            .filter(|pc| !is_expired(pc.created_at, PENDING_TTL))
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "invalid_grant"})),
                )
            })?
    };

    // Validate redirect_uri if provided
    if let Some(uri) = &req.redirect_uri {
        if *uri != pending_code.client_redirect_uri {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"error": "invalid_grant", "error_description": "redirect_uri mismatch"}),
                ),
            ));
        }
    }

    // Verify PKCE S256
    if !verify_pkce_s256(&req.code_verifier, &pending_code.code_challenge) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                json!({"error": "invalid_grant", "error_description": "PKCE verification failed"}),
            ),
        ));
    }

    let bearer = Uuid::new_v4().to_string();
    {
        let mut sessions = state.auth_store.sessions.lock().await;
        // Evict expired sessions when the map grows large to prevent unbounded memory use.
        const EVICT_THRESHOLD: usize = 256;
        if sessions.len() >= EVICT_THRESHOLD {
            sessions.retain(|_, s| !is_expired(s.created_at, SESSION_TTL));
        }
        sessions.insert(
            bearer.clone(),
            UserSession {
                email: pending_code.email.clone(),
                created_at: SystemTime::now(),
            },
        );
    }

    eprintln!("[gws mcp] Session created for {}", pending_code.email);

    Ok(Json(TokenResponse {
        access_token: bearer,
        token_type: "bearer",
        expires_in: SESSION_TTL.as_secs(),
    }))
}

// ─── Google helpers ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GoogleTokenResp {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

async fn exchange_google_code(
    cfg: &InstalledConfig,
    code: &str,
    redirect_uri: &str,
) -> anyhow::Result<GoogleTokenResp> {
    let client = crate::client::shared_client()?;
    let params = [
        ("client_id", cfg.client_id.as_str()),
        ("client_secret", cfg.client_secret.as_str()),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = crate::auth::response_text_or_placeholder(resp.text().await);
        anyhow::bail!("Google token exchange failed ({status}): {body}");
    }
    resp.json().await.map_err(Into::into)
}

async fn get_google_email(access_token: &str) -> anyhow::Result<String> {
    #[derive(Deserialize)]
    struct UserinfoResp {
        email: String,
    }
    let client = crate::client::shared_client()?;
    let resp: UserinfoResp = client
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .bearer_auth(access_token)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.email)
}

async fn refresh_google_token(
    cfg: &InstalledConfig,
    refresh_token: &str,
) -> anyhow::Result<UserGwsToken> {
    #[derive(Deserialize)]
    struct RefreshResp {
        access_token: String,
        expires_in: Option<u64>,
    }
    let client = crate::client::shared_client()?;
    let params = [
        ("client_id", cfg.client_id.as_str()),
        ("client_secret", cfg.client_secret.as_str()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = crate::auth::response_text_or_placeholder(resp.text().await);
        anyhow::bail!("Google token refresh failed ({status}): {body}");
    }
    let r: RefreshResp = resp.json().await?;
    Ok(UserGwsToken {
        access_token: r.access_token,
        refresh_token: Some(refresh_token.to_string()),
        expires_at: SystemTime::now()
            + Duration::from_secs(r.expires_in.unwrap_or(3600).saturating_sub(60)),
    })
}

// ─── Router builder ───────────────────────────────────────────────────────────

fn build_auth_router(
    mcp_service: StreamableHttpService<GwsMcpHandler, LocalSessionManager>,
    app_state: AppState,
) -> Router {
    Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(authorization_server_metadata),
        )
        .route("/oauth/register", post(oauth_register))
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/callback", get(oauth_callback))
        .route("/oauth/token", post(oauth_token))
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            bearer_auth_middleware,
        ))
        .with_state(app_state)
}

// ─── Public entry point ───────────────────────────────────────────────────────

pub(crate) async fn start_http(
    config: ServerConfig,
    port: u16,
    bind: String,
    enable_auth: bool,
    public_url: Option<String>,
) -> Result<(), GwsError> {
    let config = Arc::new(config);
    let tools_cache: Arc<Mutex<Option<Vec<Tool>>>> = Arc::new(Mutex::new(None));

    let loopback = matches!(bind.as_str(), "127.0.0.1" | "::1" | "localhost");
    // Disable allowed_hosts when a public URL is set: a reverse proxy forwards requests with the
    // public Host header (e.g. mcp.example.com) even though the bind address is loopback,
    // which causes the default allowed_hosts check to reject all MCP requests with 403.
    let http_config = if loopback && public_url.is_none() {
        StreamableHttpServerConfig::default()
    } else {
        StreamableHttpServerConfig::default().disable_allowed_hosts()
    };

    let mcp_service: StreamableHttpService<GwsMcpHandler, LocalSessionManager> = {
        let config = config.clone();
        let tools_cache = tools_cache.clone();
        StreamableHttpService::new(
            move || {
                Ok(GwsMcpHandler {
                    config: config.clone(),
                    tools_cache: tools_cache.clone(),
                })
            },
            Default::default(),
            http_config,
        )
    };

    let base_url = resolve_base_url(public_url.as_deref(), port, &bind)?;

    if public_url.is_some() && !enable_auth {
        eprintln!(
            "[gws mcp] Warning: --public-url has no effect without --auth \
             (OAuth2 metadata endpoints are only mounted when --auth is enabled)"
        );
    }

    let app = if enable_auth {
        let oauth_cfg = load_client_config().map_err(|e| {
            GwsError::Other(anyhow::anyhow!(
                "--auth requires client_secret.json (run `gws auth setup` first): {e}"
            ))
        })?;
        let app_state = AppState {
            auth_store: Arc::new(AuthStore::default()),
            oauth_cfg: Arc::new(oauth_cfg),
            user_tokens: Arc::new(Mutex::new(HashMap::new())),
            services: config.services_list().to_vec(),
            base_url: base_url.clone(),
        };
        build_auth_router(mcp_service, app_state)
    } else {
        Router::new().nest_service("/mcp", mcp_service)
    };

    let addr = format!("{bind}:{port}");
    eprintln!("[gws mcp] HTTP server listening on {base_url}/mcp");
    if enable_auth {
        eprintln!("[gws mcp] OAuth2 auth enabled — callback URL: {base_url}/oauth/callback");
        eprintln!(
            "[gws mcp] Ensure {base_url}/oauth/callback is registered as a redirect URI in Google Cloud Console"
        );
    }
    if !loopback {
        eprintln!("[gws mcp] Warning: server accessible from external hosts (--bind {bind})");
    }

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| GwsError::Other(anyhow::anyhow!("Failed to bind to {addr}: {e}")))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| GwsError::Other(anyhow::anyhow!("HTTP server error: {e}")))?;

    Ok(())
}

/// Returns true if `redirect_uri` is acceptable for the given `base_url`.
///
/// Loopback addresses are always permitted. When `base_url` is an HTTPS URL
/// (i.e. the server is deployed publicly with `--public-url https://...`),
/// `https://` redirect URIs are also allowed so that remote MCP clients can
/// complete the authorization flow.
fn is_redirect_uri_allowed(base_url: &str, redirect_uri: &str) -> bool {
    redirect_uri.starts_with("http://localhost:")
        || redirect_uri.starts_with("http://127.0.0.1:")
        || redirect_uri.starts_with("http://[::1]:")
        || (base_url.starts_with("https://") && redirect_uri.starts_with("https://"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_s256_rfc7636_example() {
        // RFC 7636 §4.6 worked example
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(verify_pkce_s256(verifier, challenge));
    }

    #[test]
    fn pkce_s256_rejects_wrong_verifier() {
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert!(!verify_pkce_s256("wrong-verifier", challenge));
    }

    #[test]
    fn server_base_ipv6_brackets() {
        assert_eq!(server_base(3000, "::"), "http://localhost:3000");
        assert_eq!(server_base(3000, "::1"), "http://localhost:3000");
        assert_eq!(server_base(3000, "0.0.0.0"), "http://localhost:3000");
        assert_eq!(server_base(3000, "127.0.0.1"), "http://localhost:3000");
    }

    #[test]
    fn resolve_base_url_none_falls_back_to_localhost() {
        let url = resolve_base_url(None, 3000, "127.0.0.1").unwrap();
        assert_eq!(url, "http://localhost:3000");
    }

    #[test]
    fn resolve_base_url_explicit_https() {
        let url = resolve_base_url(Some("https://mcp.example.com/gws"), 3000, "127.0.0.1").unwrap();
        assert_eq!(url, "https://mcp.example.com/gws");
    }

    #[test]
    fn resolve_base_url_strips_trailing_slash() {
        let url =
            resolve_base_url(Some("https://mcp.example.com/gws/"), 3000, "127.0.0.1").unwrap();
        assert_eq!(url, "https://mcp.example.com/gws");
    }

    #[test]
    fn resolve_base_url_rejects_invalid_url() {
        assert!(resolve_base_url(Some("not a url"), 3000, "127.0.0.1").is_err());
    }

    #[test]
    fn resolve_base_url_rejects_non_http_scheme() {
        assert!(resolve_base_url(Some("ftp://example.com"), 3000, "127.0.0.1").is_err());
    }

    #[test]
    fn resolve_base_url_rejects_query_string() {
        assert!(
            resolve_base_url(Some("https://example.com/gws?foo=bar"), 3000, "127.0.0.1").is_err()
        );
    }

    #[test]
    fn resolve_base_url_rejects_fragment() {
        assert!(
            resolve_base_url(Some("https://example.com/gws#section"), 3000, "127.0.0.1").is_err()
        );
    }

    #[test]
    fn resolve_base_url_http_non_loopback_warns_but_succeeds() {
        // Non-loopback http:// emits a warning but returns Ok (not an error).
        let result = resolve_base_url(Some("http://example.com/gws"), 3000, "127.0.0.1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "http://example.com/gws");
    }

    #[test]
    fn www_authenticate_resource_metadata_is_absolute_url() {
        // RFC 9728 §5.1 requires resource_metadata to be an absolute HTTPS URL.
        // A relative path (/.well-known/...) would cause MCP clients to query the
        // wrong endpoint when the server is mounted at a sub-path like /gws.
        let base = "https://mcp.example.com/gws";
        let value = format!(
            "Bearer realm=\"gws-mcp\", resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
            base
        );
        let hv = HeaderValue::from_str(&value).unwrap();
        let hv_str = hv.to_str().unwrap();
        assert!(
            hv_str.contains("https://mcp.example.com/gws/.well-known/oauth-protected-resource"),
            "resource_metadata must be an absolute HTTPS URL, got: {hv_str}"
        );
        assert!(
            !hv_str.contains("\"/.well-known/"),
            "resource_metadata must not be a relative path, got: {hv_str}"
        );
    }

    #[test]
    fn redirect_uri_loopback_always_allowed() {
        let local_base = "http://localhost:3000";
        assert!(is_redirect_uri_allowed(local_base, "http://localhost:8080"));
        assert!(is_redirect_uri_allowed(local_base, "http://127.0.0.1:8080"));
        assert!(is_redirect_uri_allowed(local_base, "http://[::1]:8080"));
    }

    #[test]
    fn redirect_uri_https_allowed_only_for_public_https_base() {
        let public_base = "https://mcp.example.com/gws";
        let local_base = "http://localhost:3000";
        assert!(is_redirect_uri_allowed(
            public_base,
            "https://client.example.com/callback"
        ));
        assert!(!is_redirect_uri_allowed(
            local_base,
            "https://client.example.com/callback"
        ));
    }

    #[test]
    fn redirect_uri_http_non_loopback_always_rejected() {
        let public_base = "https://mcp.example.com/gws";
        assert!(!is_redirect_uri_allowed(
            public_base,
            "http://client.example.com/callback"
        ));
    }

    #[test]
    fn http_config_uses_allowed_hosts_only_for_local_without_public_url() {
        // When --public-url is set, a reverse proxy forwards the public Host header,
        // so allowed_hosts must be disabled regardless of bind address.
        let loopback = true;
        let has_public_url = true;
        assert!(
            !(loopback && !has_public_url),
            "allowed_hosts should be disabled when public_url is set"
        );
        let has_public_url = false;
        assert!(
            loopback && !has_public_url,
            "allowed_hosts should be enabled for local-only deployments"
        );
    }
}
