use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Json, Response},
    routing::{get, post},
    Router,
};
use dashmap::DashMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Debug)]
struct GatewayState {
    upstream: String,
    rate_limit: DashMap<String, u32>,
    secret: String,
}

type AppState = Arc<GatewayState>;

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    if path == "/health" || path == "/api/v1/lb/health" {
        return Ok(next.run(req).await);
    }
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token {
        Some(tok) => {
            let key = DecodingKey::from_secret(state.secret.as_bytes());
            match decode::<Claims>(tok, &key, &Validation::default()) {
                Ok(data) => {
                    let mut count = state.rate_limit.entry(data.claims.sub).or_insert(0);
                    *count += 1;
                    if *count > 1000 {
                        return Err(StatusCode::TOO_MANY_REQUESTS);
                    }
                }
                Err(_) => return Err(StatusCode::UNAUTHORIZED),
            }
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    }
    Ok(next.run(req).await)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    upstream: String,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "alice-lb-gateway",
        upstream: state.upstream.clone(),
    })
}

async fn proxy_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let url = format!("{}{}", state.upstream, path);
    let client = reqwest::Client::new();
    let method = reqwest::Method::from_bytes(req.method().as_str().as_bytes())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let resp = client
        .request(method, &url)
        .body(body_bytes.to_vec())
        .header("content-type", "application/json")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let bytes = resp.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(bytes))
        .unwrap())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let upstream = std::env::var("UPSTREAM_URL")
        .unwrap_or_else(|_| "http://localhost:8126".to_string());
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "alice-lb-secret".to_string());

    let state: AppState = Arc::new(GatewayState {
        upstream,
        rate_limit: DashMap::new(),
        secret,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/lb/backends", post(proxy_handler))
        .route("/api/v1/lb/route", post(proxy_handler))
        .route("/api/v1/lb/health", get(proxy_handler))
        .route("/api/v1/lb/drain", post(proxy_handler))
        .route("/api/v1/lb/stats", get(proxy_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port: u16 = std::env::var("GATEWAY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9126);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("alice-lb-gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
