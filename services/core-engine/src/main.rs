use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
struct BackendNode {
    id: String,
    address: String,
    port: u16,
    weight: u32,
    healthy: bool,
    draining: bool,
    active_connections: u32,
}

#[derive(Debug, Default)]
struct Stats {
    total_ops: u64,
    backend_ops: u64,
    route_ops: u64,
    drain_ops: u64,
}

#[derive(Debug, Default)]
struct AppData {
    stats: Stats,
    backends: HashMap<String, Vec<BackendNode>>,
    routes: Vec<RouteRule>,
    total_requests_routed: u64,
}

#[derive(Debug, Clone, Serialize)]
struct RouteRule {
    id: String,
    host: String,
    path_prefix: String,
    pool: String,
    algorithm: String,
}

type AppState = Arc<Mutex<AppData>>;

// --- /api/v1/lb/backends ---
#[derive(Debug, Deserialize)]
struct BackendsRequest {
    pool: String,
    nodes: Vec<NodeSpec>,
}

#[derive(Debug, Deserialize)]
struct NodeSpec {
    address: String,
    port: u16,
    #[serde(default = "default_weight")]
    weight: u32,
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Serialize)]
struct BackendsResponse {
    request_id: String,
    pool: String,
    nodes_registered: usize,
    nodes: Vec<BackendNode>,
}

async fn register_backends(
    State(state): State<AppState>,
    Json(req): Json<BackendsRequest>,
) -> Result<Json<BackendsResponse>, StatusCode> {
    let nodes: Vec<BackendNode> = req
        .nodes
        .iter()
        .map(|n| BackendNode {
            id: Uuid::new_v4().to_string(),
            address: n.address.clone(),
            port: n.port,
            weight: n.weight,
            healthy: true,
            draining: false,
            active_connections: 0,
        })
        .collect();

    let count = nodes.len();
    {
        let mut d = state.lock().unwrap();
        d.stats.total_ops += 1;
        d.stats.backend_ops += 1;
        d.backends.insert(req.pool.clone(), nodes.clone());
    }

    Ok(Json(BackendsResponse {
        request_id: Uuid::new_v4().to_string(),
        pool: req.pool,
        nodes_registered: count,
        nodes,
    }))
}

// --- /api/v1/lb/route ---
#[derive(Debug, Deserialize)]
struct RouteRequest {
    host: String,
    path_prefix: String,
    pool: String,
    #[serde(default = "default_algo")]
    algorithm: String,
}

fn default_algo() -> String {
    "round_robin".to_string()
}

#[derive(Debug, Serialize)]
struct RouteResponse {
    request_id: String,
    rule_id: String,
    host: String,
    path_prefix: String,
    pool: String,
    algorithm: String,
    selected_backend: Option<String>,
}

async fn configure_route(
    State(state): State<AppState>,
    Json(req): Json<RouteRequest>,
) -> Result<Json<RouteResponse>, StatusCode> {
    let rule_id = Uuid::new_v4().to_string();
    let selected = {
        let mut d = state.lock().unwrap();
        d.stats.total_ops += 1;
        d.stats.route_ops += 1;
        d.total_requests_routed += 1;
        let rule = RouteRule {
            id: rule_id.clone(),
            host: req.host.clone(),
            path_prefix: req.path_prefix.clone(),
            pool: req.pool.clone(),
            algorithm: req.algorithm.clone(),
        };
        d.routes.push(rule);
        // select backend by round-robin index
        d.backends.get(&req.pool).and_then(|nodes| {
            let healthy: Vec<&BackendNode> = nodes.iter().filter(|n| n.healthy && !n.draining).collect();
            let idx = (d.total_requests_routed as usize - 1) % healthy.len().max(1);
            healthy.get(idx).map(|n| format!("{}:{}", n.address, n.port))
        })
    };

    Ok(Json(RouteResponse {
        request_id: Uuid::new_v4().to_string(),
        rule_id,
        host: req.host,
        path_prefix: req.path_prefix,
        pool: req.pool,
        algorithm: req.algorithm,
        selected_backend: selected,
    }))
}

// --- /api/v1/lb/health ---
#[derive(Debug, Serialize)]
struct LbHealthResponse {
    status: &'static str,
    pools: Vec<PoolHealth>,
    total_healthy: usize,
    total_nodes: usize,
}

#[derive(Debug, Serialize)]
struct PoolHealth {
    pool: String,
    total: usize,
    healthy: usize,
    draining: usize,
}

async fn lb_health(State(state): State<AppState>) -> Json<LbHealthResponse> {
    let d = state.lock().unwrap();
    let mut pools = Vec::new();
    let mut total_healthy = 0;
    let mut total_nodes = 0;
    for (pool, nodes) in &d.backends {
        let h = nodes.iter().filter(|n| n.healthy && !n.draining).count();
        let dr = nodes.iter().filter(|n| n.draining).count();
        total_healthy += h;
        total_nodes += nodes.len();
        pools.push(PoolHealth {
            pool: pool.clone(),
            total: nodes.len(),
            healthy: h,
            draining: dr,
        });
    }
    Json(LbHealthResponse {
        status: if total_nodes == 0 || total_healthy > 0 { "ok" } else { "degraded" },
        pools,
        total_healthy,
        total_nodes,
    })
}

// --- /api/v1/lb/drain ---
#[derive(Debug, Deserialize)]
struct DrainRequest {
    pool: String,
    node_id: String,
}

#[derive(Debug, Serialize)]
struct DrainResponse {
    request_id: String,
    pool: String,
    node_id: String,
    draining: bool,
    message: String,
}

async fn drain_node(
    State(state): State<AppState>,
    Json(req): Json<DrainRequest>,
) -> Result<Json<DrainResponse>, StatusCode> {
    let mut d = state.lock().unwrap();
    d.stats.total_ops += 1;
    d.stats.drain_ops += 1;

    let found = d
        .backends
        .get_mut(&req.pool)
        .and_then(|nodes| nodes.iter_mut().find(|n| n.id == req.node_id))
        .map(|n| {
            n.draining = true;
            true
        })
        .unwrap_or(false);

    if !found {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(DrainResponse {
        request_id: Uuid::new_v4().to_string(),
        pool: req.pool,
        node_id: req.node_id,
        draining: true,
        message: "node marked for draining; new connections will be redirected".to_string(),
    }))
}

// --- /api/v1/lb/stats ---
#[derive(Debug, Serialize)]
struct StatsResponse {
    service: &'static str,
    version: &'static str,
    total_ops: u64,
    backend_ops: u64,
    route_ops: u64,
    drain_ops: u64,
    total_requests_routed: u64,
    registered_pools: usize,
    route_rules: usize,
}

async fn get_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let d = state.lock().unwrap();
    Json(StatsResponse {
        service: "alice-lb-core",
        version: env!("CARGO_PKG_VERSION"),
        total_ops: d.stats.total_ops,
        backend_ops: d.stats.backend_ops,
        route_ops: d.stats.route_ops,
        drain_ops: d.stats.drain_ops,
        total_requests_routed: d.total_requests_routed,
        registered_pools: d.backends.len(),
        route_rules: d.routes.len(),
    })
}

// --- /health ---
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    uptime_secs: u64,
    total_ops: u64,
}

async fn health(
    State(state): State<AppState>,
    axum::extract::Extension(start): axum::extract::Extension<Arc<Instant>>,
) -> Json<HealthResponse> {
    let d = state.lock().unwrap();
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: start.elapsed().as_secs(),
        total_ops: d.stats.total_ops,
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let state: AppState = Arc::new(Mutex::new(AppData::default()));
    let start = Arc::new(Instant::now());

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/lb/backends", post(register_backends))
        .route("/api/v1/lb/route", post(configure_route))
        .route("/api/v1/lb/health", get(lb_health))
        .route("/api/v1/lb/drain", post(drain_node))
        .route("/api/v1/lb/stats", get(get_stats))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(axum::extract::Extension(start))
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8126);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("alice-lb-core listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
