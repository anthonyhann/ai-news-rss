use anyhow::{Context, Result};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::config::AppConfig;
use crate::pipeline::Pipeline;
use crate::store::Store;

#[derive(Clone)]
struct AppState {
    cfg: AppConfig,
    store: Store,
    public_dir: PathBuf,
}

pub async fn serve(cfg: AppConfig) -> Result<()> {
    let store = Store::open(cfg.db_path()).await?;
    let state = Arc::new(AppState {
        public_dir: cfg.public_dir(),
        cfg: cfg.clone(),
        store,
    });
    let api = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/feed", get(feed))
        .route("/api/v1/sources", get(sources))
        .route("/api/v1/briefs", get(briefs))
        .route("/api/v1/briefs/{date}", get(briefs_for_date))
        .route("/api/v1/actions", get(actions))
        .route("/api/v1/runs", get(runs))
        .route("/api/v1/admin/run", post(admin_run))
        .with_state(state.clone());
    let app = Router::new()
        .merge(api)
        .nest_service("/data", ServeDir::new(state.public_dir.clone()))
        .fallback_service(ServeDir::new("web/dist").append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&cfg.server.bind)
        .await
        .with_context(|| format!("bind {}", cfg.server.bind))?;
    tracing::info!(bind = %cfg.server.bind, "ainews server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "ainews",
        "schema_version": 1,
        "data_dir": state.cfg.data_dir(),
        "public_dir": state.public_dir,
    }))
}

async fn feed(State(state): State<Arc<AppState>>) -> Response {
    read_json(&state.public_dir.join("latest.json")).await
}

async fn sources(State(state): State<Arc<AppState>>) -> Response {
    match state.store.source_health(500).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => internal(error),
    }
}

async fn briefs(State(state): State<Arc<AppState>>) -> Response {
    match state.store.list_briefs(100).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => internal(error),
    }
}

async fn briefs_for_date(State(state): State<Arc<AppState>>, Path(date): Path<String>) -> Response {
    match state.store.get_briefs_for_date(&date).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => internal(error),
    }
}

async fn actions(State(state): State<Arc<AppState>>) -> Response {
    let path = state.public_dir.join("actions.json");
    if path.exists() {
        read_json(&path).await
    } else {
        Json(json!({"date": crate::pipeline::local_date(), "top_action": null})).into_response()
    }
}

async fn runs(State(state): State<Arc<AppState>>) -> Response {
    read_json(&state.public_dir.join("runs/latest.json")).await
}

async fn admin_run(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Ok(expected) = std::env::var(&state.cfg.server.admin_token_env) {
        if !expected.trim().is_empty() {
            let got = headers
                .get("x-admin-token")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            if got != expected {
                return (StatusCode::UNAUTHORIZED, "invalid admin token").into_response();
            }
        }
    }
    let cfg = state.cfg.clone();
    tokio::spawn(async move {
        match Pipeline::new(cfg, false).await {
            Ok(pipeline) => {
                if let Err(error) = pipeline.run(false, true, "manual").await {
                    tracing::error!(%error, "manual run failed");
                }
            }
            Err(error) => tracing::error!(%error, "manual pipeline init failed"),
        }
    });
    Json(json!({"ok": true, "status": "started"})).into_response()
}

async fn read_json(path: &std::path::Path) -> Response {
    match tokio::fs::read_to_string(path).await {
        Ok(value) => match serde_json::from_str::<Value>(&value) {
            Ok(value) => Json(value).into_response(),
            Err(error) => internal(error),
        },
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                (StatusCode::NOT_FOUND, "data not generated yet").into_response()
            } else {
                internal(error)
            }
        }
    }
}

fn internal(error: impl std::fmt::Display) -> Response {
    tracing::error!(%error, "api error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"ok": false, "error": error.to_string()})),
    )
        .into_response()
}
