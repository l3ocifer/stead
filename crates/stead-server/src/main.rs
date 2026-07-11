//! stead-server: HTTP API (and later, MCP over stdio/SSE) for the
//! site store. Spatial query tools will follow mazzap's region
//! grammar — one `region` argument per tool, points accept lat/lon or
//! scene-local meters and outputs echo both (docs/prior-art-mazzap.md).

use axum::{routing::get, Json, Router};

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "stead-server",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "ok",
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app = Router::new().route("/healthz", get(health));
    let addr = std::env::var("STEAD_BIND").unwrap_or_else(|_| "127.0.0.1:4180".into());
    tracing::info!(%addr, "stead-server listening");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
