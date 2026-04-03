use axum::{
    routing::get,
    response::Json,
    Router,
};
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize)]
struct Health { status: String, service: String, version: String }

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lb-engine=info".into()),
        )
        .init();

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/lb/health", get(health))
        // TODO: Add domain-specific endpoints here
        .layer(cors);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("ALICE LB Core Engine on {addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        service: "ALICE LB".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn health_check() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    }
}
