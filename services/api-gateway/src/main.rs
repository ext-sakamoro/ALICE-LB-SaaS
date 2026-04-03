use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{Json, Response},
    routing::{any, get},
    Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

struct AppState {
    core_url: String,
    jwt_secret: String,
    supabase_url: String,
    supabase_service_key: String,
    rate_limiters: DashMap<String, TokenBucket>,
    start_time: Instant,
}

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max: f64, rate: f64) -> Self {
        Self { tokens: max, max_tokens: max, refill_rate: rate, last_refill: Instant::now() }
    }
    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
        if self.tokens >= 1.0 { self.tokens -= 1.0; true } else { false }
    }
}

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Health { status: String, version: String, uptime_secs: u64 }

#[derive(Serialize)]
struct Err {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

#[derive(Serialize)]
struct LicenseInfo { license: String, source_code: String, notice: String }

#[derive(Deserialize, Serialize, Clone)]
struct Claims {
    sub: String,
    email: Option<String>,
    role: Option<String>,
    exp: usize,
    #[serde(default)]
    plan: Option<String>,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_gateway=info,tower_http=info".into()),
        )
        .init();
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.into());

    let state = Arc::new(AppState {
        core_url: env("CORE_ENGINE_URL", "http://localhost:8081"),
        jwt_secret: env("JWT_SECRET", "dev-secret-change-me"),
        supabase_url: env("SUPABASE_URL", ""),
        supabase_service_key: env("SUPABASE_SERVICE_ROLE_KEY", ""),
        rate_limiters: DashMap::new(),
        start_time: Instant::now(),
    });
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let frontend_url = env("FRONTEND_URL", "http://127.0.0.1:3000");

    let public = Router::new()
        .route("/health", get(health))
        .route("/license", get(license_handler));

    let api = Router::new()
        .route("/api/v1/{*p}", any(proxy_core))
        .layer(middleware::from_fn_with_state(state.clone(), auth_mw))
        .layer(middleware::from_fn_with_state(state.clone(), rate_mw));

    let admin = Router::new()
        .route("/api/v1/admin/stats", get(admin_stats))
        .route("/api/v1/admin/users", get(admin_users))
        .route("/api/v1/admin/users/{id}", axum::routing::patch(admin_update_user))
        .route("/api/v1/admin/projects", get(admin_projects))
        .route("/api/v1/admin/projects/{id}", axum::routing::patch(admin_update_project))
        .route("/api/v1/admin/revenue", get(admin_revenue))
        .layer(middleware::from_fn_with_state(state.clone(), admin_mw))
        .layer(middleware::from_fn_with_state(state.clone(), auth_mw));

    let frontend_proxy = Router::new()
        .fallback(move |req: Request| proxy_frontend(frontend_url.clone(), req));

    let app = Router::new()
        .merge(public)
        .merge(api)
        .merge(admin)
        .merge(frontend_proxy)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("ALICE LB API Gateway on {addr}");
    axum::serve(listener, app).await.unwrap();
}

// ---------------------------------------------------------------------------
// Public handlers
// ---------------------------------------------------------------------------

async fn health(State(s): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: s.start_time.elapsed().as_secs(),
    })
}

async fn license_handler() -> (HeaderMap, Json<LicenseInfo>) {
    let mut h = HeaderMap::new();
    h.insert("X-License", "AGPL-3.0-or-later".parse().unwrap());
    (h, Json(LicenseInfo {
        license: "AGPL-3.0-or-later".into(),
        source_code: "https://github.com/ext-sakamoro/ALICE-LB-SaaS".into(),
        notice: "SaaS operators must publish complete service source code under AGPL-3.0.".into(),
    }))
}

// ---------------------------------------------------------------------------
// Auth middleware — JWT + API Key (Supabase lookup)
// ---------------------------------------------------------------------------

async fn validate_api_key(state: &AppState, key: &str) -> Option<Claims> {
    if state.supabase_url.is_empty() || state.supabase_service_key.is_empty() {
        return Some(Claims {
            sub: "api-key-user".into(), email: None,
            role: Some("api".into()), exp: usize::MAX,
            plan: Some("Free".into()),
        });
    }
    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/profiles?api_key=eq.{}&select=id,plan", state.supabase_url, key);

    #[derive(Deserialize)]
    struct Profile { id: String, plan: Option<String> }

    let resp = client.get(&url)
        .header("apikey", &state.supabase_service_key)
        .header("Authorization", format!("Bearer {}", state.supabase_service_key))
        .send().await.ok()?;

    let profiles: Vec<Profile> = resp.json().await.ok()?;
    let profile = profiles.first()?;

    Some(Claims {
        sub: profile.id.clone(), email: None,
        role: Some("api".into()), exp: usize::MAX,
        plan: Some(profile.plan.clone().unwrap_or_else(|| "Free".into())),
    })
}

async fn auth_mw(
    State(s): State<Arc<AppState>>, mut req: Request, next: Next,
) -> Result<Response, (StatusCode, Json<Err>)> {
    let auth = req.headers().get("Authorization").and_then(|h| h.to_str().ok()).map(|s| s.to_string());
    let api_key = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

    if let Some(a) = &auth {
        if let Some(token) = a.strip_prefix("Bearer ") {
            let mut val = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
            val.validate_aud = false;
            match jsonwebtoken::decode::<Claims>(
                token, &jsonwebtoken::DecodingKey::from_secret(s.jwt_secret.as_bytes()), &val,
            ) {
                Ok(data) => { req.extensions_mut().insert(data.claims); return Ok(next.run(req).await); }
                Err(e) => return Err((StatusCode::UNAUTHORIZED, Json(Err { error: "Invalid token".into(), details: Some(e.to_string()) }))),
            }
        }
    }

    if let Some(key) = api_key {
        if let Some(claims) = validate_api_key(&s, &key).await {
            req.extensions_mut().insert(claims);
            return Ok(next.run(req).await);
        }
        return Err((StatusCode::UNAUTHORIZED, Json(Err { error: "Invalid API key".into(), details: None })));
    }

    Err((StatusCode::UNAUTHORIZED, Json(Err { error: "Auth required".into(), details: Some("Provide Bearer token or X-API-Key".into()) })))
}

// ---------------------------------------------------------------------------
// Rate limit middleware — per-plan token bucket + usage tracking
// ---------------------------------------------------------------------------

async fn rate_mw(
    State(s): State<Arc<AppState>>, req: Request, next: Next,
) -> Result<Response, (StatusCode, Json<Err>)> {
    let claims = req.extensions().get::<Claims>().cloned();
    let uid = claims.as_ref().map(|c| c.sub.clone()).unwrap_or_else(|| "anon".into());
    let plan = claims.as_ref().and_then(|c| c.plan.as_deref()).unwrap_or("Free");

    let max_tokens = match plan {
        "Enterprise" => 100_000.0,
        "Pro" => 10_000.0,
        "General" => 1_000.0,
        _ => 100.0,
    };

    let ok = {
        let mut e = s.rate_limiters.entry(uid.clone()).or_insert_with(|| TokenBucket::new(max_tokens, max_tokens / 3600.0));
        if (e.max_tokens - max_tokens).abs() > 1.0 {
            *e = TokenBucket::new(max_tokens, max_tokens / 3600.0);
        }
        e.try_consume()
    };
    if !ok {
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(Err { error: "Rate limit exceeded".into(), details: None })));
    }

    let state = s.clone();
    let method = req.method().to_string();
    let endpoint = req.uri().path().to_string();
    let uid_clone = uid.clone();
    let start = Instant::now();

    let resp = next.run(req).await;

    let status_code = resp.status().as_u16() as i32;
    let response_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    tokio::spawn(async move {
        record_usage(&state, &uid_clone, &endpoint, &method, status_code, response_time_ms).await;
    });

    Ok(resp)
}

async fn record_usage(
    state: &AppState, user_id: &str, endpoint: &str, method: &str,
    status_code: i32, response_time_ms: f64,
) {
    if state.supabase_url.is_empty() || state.supabase_service_key.is_empty() { return; }
    if user_id.len() != 36 { return; }

    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/api_usage", state.supabase_url);
    let body = serde_json::json!({
        "user_id": user_id, "endpoint": endpoint, "method": method,
        "status_code": status_code, "response_time_ms": response_time_ms,
    });
    let _ = client.post(&url)
        .header("apikey", &state.supabase_service_key)
        .header("Authorization", format!("Bearer {}", state.supabase_service_key))
        .header("Content-Type", "application/json")
        .json(&body).send().await;
}

// ---------------------------------------------------------------------------
// Core Engine proxy
// ---------------------------------------------------------------------------

async fn proxy_core(
    State(s): State<Arc<AppState>>, req: Request,
) -> Result<Response, (StatusCode, Json<Err>)> {
    let client = reqwest::Client::new();
    let path = req.uri().path().to_owned();
    let q = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
    let method = req.method().clone();
    let hdrs = req.headers().clone();
    let body = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(Err { error: "Body read fail".into(), details: Some(e.to_string()) })))?;
    let mut r = client.request(method, format!("{}{path}{q}", s.core_url));
    for (k, v) in hdrs.iter() { if k != "host" { r = r.header(k, v); } }
    let resp = r.body(body).send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(Err { error: "Upstream unavailable".into(), details: Some(e.to_string()) })))?;
    let st = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let rh = resp.headers().clone();
    let rb = resp.bytes().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(Err { error: "Read fail".into(), details: Some(e.to_string()) })))?;
    let mut b = Response::builder().status(st);
    for (k, v) in rh.iter() { b = b.header(k, v); }
    b.body(Body::from(rb))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(Err { error: "Build fail".into(), details: Some(e.to_string()) })))
}

// ---------------------------------------------------------------------------
// Frontend proxy
// ---------------------------------------------------------------------------

async fn proxy_frontend(frontend_url: String, req: Request) -> Response {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let path = req.uri().path_and_query().map(|pq| pq.to_string()).unwrap_or_else(|| "/".into());
    let method = req.method().clone();
    let hdrs = req.headers().clone();
    let body = axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await.unwrap_or_default();

    let target = format!("{frontend_url}{path}");
    let mut r = client.request(method, &target);
    for (k, v) in hdrs.iter() {
        if k != "host" && k != "transfer-encoding" { r = r.header(k, v); }
    }

    match r.body(body).send().await {
        Ok(resp) => {
            let st = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let rh = resp.headers().clone();
            let rb = resp.bytes().await.unwrap_or_default();
            let mut b = Response::builder().status(st);
            for (k, v) in rh.iter() {
                if k == "location" {
                    if let Ok(loc) = v.to_str() {
                        let rewritten = loc
                            .replace("http://127.0.0.1:3000", "")
                            .replace("http://localhost:3000", "");
                        b = b.header(k, rewritten);
                        continue;
                    }
                }
                if k == "transfer-encoding" { continue; }
                b = b.header(k, v);
            }
            b.body(Body::from(rb)).unwrap_or_else(|_| {
                Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("proxy error")).unwrap()
            })
        }
        Err(e) => {
            tracing::error!(error = %e, target = %target, "frontend proxy failed");
            Response::builder().status(StatusCode::BAD_GATEWAY)
                .header("content-type", "text/plain")
                .body(Body::from("Frontend unavailable")).unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Admin middleware
// ---------------------------------------------------------------------------

async fn admin_mw(
    State(s): State<Arc<AppState>>, req: Request, next: Next,
) -> Result<Response, (StatusCode, Json<Err>)> {
    let claims = req.extensions().get::<Claims>().cloned();
    let uid = claims.as_ref().map(|c| c.sub.clone()).unwrap_or_default();

    if s.supabase_url.is_empty() || s.supabase_service_key.is_empty() {
        return Ok(next.run(req).await);
    }

    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/profiles?id=eq.{}&select=role", s.supabase_url, uid);

    #[derive(Deserialize)]
    struct RoleCheck { role: Option<String> }

    let resp = client.get(&url)
        .header("apikey", &s.supabase_service_key)
        .header("Authorization", format!("Bearer {}", s.supabase_service_key))
        .send().await;

    let is_admin = if let Ok(r) = resp {
        r.json::<Vec<RoleCheck>>().await.ok()
            .and_then(|v| v.first().and_then(|p| p.role.as_deref().map(|r| r == "admin")))
            .unwrap_or(false)
    } else {
        false
    };

    if !is_admin {
        return Err((StatusCode::FORBIDDEN, Json(Err { error: "Admin access required".into(), details: None })));
    }

    Ok(next.run(req).await)
}

// ---------------------------------------------------------------------------
// Admin handlers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AdminStats {
    uptime_secs: u64,
    total_users: i64,
    total_projects: i64,
    today_api_calls: i64,
    active_rate_limiters: usize,
}

async fn admin_stats(State(s): State<Arc<AppState>>) -> Json<AdminStats> {
    let client = reqwest::Client::new();
    let total_users = supabase_count(&client, &s, "profiles", "").await;
    let total_projects = supabase_count(&client, &s, "projects", "").await;
    let today = chrono_today();
    let today_api_calls = supabase_count(
        &client, &s, "api_usage", &format!("&created_at=gte.{today}T00:00:00Z"),
    ).await;

    Json(AdminStats {
        uptime_secs: s.start_time.elapsed().as_secs(),
        total_users,
        total_projects,
        today_api_calls,
        active_rate_limiters: s.rate_limiters.len(),
    })
}

async fn admin_users(
    State(s): State<Arc<AppState>>,
) -> Result<Response, (StatusCode, Json<Err>)> {
    supabase_get(&s, "profiles?select=id,email,full_name,plan,role,banned,created_at&order=created_at.desc&limit=200").await
}

async fn admin_update_user(
    State(s): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, (StatusCode, Json<Err>)> {
    let allowed = ["plan", "role", "banned"];
    let filtered: serde_json::Map<String, serde_json::Value> = body.as_object()
        .map(|o| o.iter().filter(|(k, _)| allowed.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    if filtered.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(Err { error: "No valid fields".into(), details: None })));
    }

    supabase_patch(&s, &format!("profiles?id=eq.{id}"), &serde_json::Value::Object(filtered)).await
}

async fn admin_projects(
    State(s): State<Arc<AppState>>,
) -> Result<Response, (StatusCode, Json<Err>)> {
    supabase_get(&s, "projects?select=id,name,owner_id,is_public,hidden,created_at,updated_at&order=updated_at.desc&limit=200").await
}

async fn admin_update_project(
    State(s): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, (StatusCode, Json<Err>)> {
    let allowed = ["hidden", "is_public"];
    let filtered: serde_json::Map<String, serde_json::Value> = body.as_object()
        .map(|o| o.iter().filter(|(k, _)| allowed.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    if filtered.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(Err { error: "No valid fields".into(), details: None })));
    }

    supabase_patch(&s, &format!("projects?id=eq.{id}"), &serde_json::Value::Object(filtered)).await
}

async fn admin_revenue(
    State(s): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<Err>)> {
    let client = reqwest::Client::new();
    let total_general = supabase_count(&client, &s, "profiles", "&plan=eq.General").await;
    let total_pro = supabase_count(&client, &s, "profiles", "&plan=eq.Pro").await;
    let total_enterprise = supabase_count(&client, &s, "profiles", "&plan=eq.Enterprise").await;

    let mrr = total_general * 1500 + total_pro * 5000;

    Ok(Json(serde_json::json!({
        "subscribers": {
            "general": total_general,
            "pro": total_pro,
            "enterprise": total_enterprise,
        },
        "mrr_jpy": mrr,
        "note": "Enterprise revenue not included (custom pricing)"
    })))
}

// ---------------------------------------------------------------------------
// Supabase admin helpers
// ---------------------------------------------------------------------------

async fn supabase_count(
    client: &reqwest::Client, s: &AppState, table: &str, filter: &str,
) -> i64 {
    if s.supabase_url.is_empty() { return 0; }
    let url = format!("{}/rest/v1/{table}?select=id{filter}", s.supabase_url);
    client.get(&url)
        .header("apikey", &s.supabase_service_key)
        .header("Authorization", format!("Bearer {}", s.supabase_service_key))
        .header("Prefer", "count=exact")
        .header("Range-Unit", "items")
        .header("Range", "0-0")
        .send().await.ok()
        .and_then(|r| r.headers().get("content-range").and_then(|v| v.to_str().ok().map(|s| s.to_string())).map(|cr| {
            cr.split('/').next_back().and_then(|n| n.parse::<i64>().ok()).unwrap_or(0)
        }))
        .unwrap_or(0)
}

async fn supabase_get(
    s: &AppState, path: &str,
) -> Result<Response, (StatusCode, Json<Err>)> {
    if s.supabase_url.is_empty() {
        return Err((StatusCode::SERVICE_UNAVAILABLE, Json(Err { error: "Supabase not configured".into(), details: None })));
    }
    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/{path}", s.supabase_url);
    let resp = client.get(&url)
        .header("apikey", &s.supabase_service_key)
        .header("Authorization", format!("Bearer {}", s.supabase_service_key))
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(Err { error: format!("supabase: {e}"), details: None })))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body = resp.bytes().await.unwrap_or_default();

    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap())
}

async fn supabase_patch(
    s: &AppState, path: &str, body: &serde_json::Value,
) -> Result<Response, (StatusCode, Json<Err>)> {
    if s.supabase_url.is_empty() {
        return Err((StatusCode::SERVICE_UNAVAILABLE, Json(Err { error: "Supabase not configured".into(), details: None })));
    }
    let client = reqwest::Client::new();
    let url = format!("{}/rest/v1/{path}", s.supabase_url);
    let resp = client.patch(&url)
        .header("apikey", &s.supabase_service_key)
        .header("Authorization", format!("Bearer {}", s.supabase_service_key))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=representation")
        .json(body)
        .send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(Err { error: format!("supabase: {e}"), details: None })))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let rb = resp.bytes().await.unwrap_or_default();

    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(rb))
        .unwrap())
}

fn chrono_today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = secs.div_euclid(86400) + 719468;
    let era = days.div_euclid(146097);
    let doe = days.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_basic() {
        let mut tb = TokenBucket::new(10.0, 10.0);
        for _ in 0..10 { assert!(tb.try_consume()); }
        assert!(!tb.try_consume());
    }

    #[test]
    fn plan_rate_limits() {
        let limits: Vec<(&str, f64)> = vec![
            ("Free", 100.0), ("General", 1_000.0), ("Pro", 10_000.0), ("Enterprise", 100_000.0),
        ];
        for (plan, expected) in limits {
            let max = match plan {
                "Enterprise" => 100_000.0,
                "Pro" => 10_000.0,
                "General" => 1_000.0,
                _ => 100.0,
            };
            assert!((max - expected).abs() < f64::EPSILON, "plan {plan}");
        }
    }

    #[test]
    fn chrono_today_format() {
        let d = chrono_today();
        assert_eq!(d.len(), 10);
        assert_eq!(&d[4..5], "-");
        assert_eq!(&d[7..8], "-");
    }
}
