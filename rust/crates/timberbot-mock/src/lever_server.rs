use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::SharedMockState;

/// Mock of Timberborn's official 1.0+ built-in HTTP Lever / HTTP Adapter API (`localhost:8080/api/`).
pub fn build_router(state: Arc<SharedMockState>) -> Router {
    Router::new()
        .route("/api/levers", get(list_levers))
        .route("/api/levers/:name", get(get_lever))
        .route("/api/switch-on/:name", get(switch_on).post(switch_on))
        .route("/api/switch-off/:name", get(switch_off).post(switch_off))
        .route("/api/color/:name/:hex", get(set_color).post(set_color))
        .route("/api/adapters", get(list_adapters))
        .route("/api/adapters/:name", get(get_adapter))
        .with_state(state)
}

async fn list_levers(State(s): State<Arc<SharedMockState>>) -> Response {
    let w = s.world.lock().expect("lock");
    Json(w.levers.clone()).into_response()
}

async fn get_lever(State(s): State<Arc<SharedMockState>>, Path(name): Path<String>) -> Response {
    let w = s.world.lock().expect("lock");
    match w.levers.iter().find(|l| l.name == name) {
        Some(l) => Json(l.clone()).into_response(),
        None => (StatusCode::NOT_FOUND, format!("lever '{name}' not found")).into_response(),
    }
}

async fn switch_on(State(s): State<Arc<SharedMockState>>, Path(name): Path<String>) -> Response {
    let mut w = s.world.lock().expect("lock");
    match w.levers.iter_mut().find(|l| l.name == name) {
        Some(l) => {
            l.state = true;
            (StatusCode::OK, "ok").into_response()
        }
        None => (StatusCode::NOT_FOUND, format!("lever '{name}' not found")).into_response(),
    }
}

async fn switch_off(State(s): State<Arc<SharedMockState>>, Path(name): Path<String>) -> Response {
    let mut w = s.world.lock().expect("lock");
    match w.levers.iter_mut().find(|l| l.name == name) {
        Some(l) => {
            l.state = false;
            (StatusCode::OK, "ok").into_response()
        }
        None => (StatusCode::NOT_FOUND, format!("lever '{name}' not found")).into_response(),
    }
}

async fn set_color(
    State(s): State<Arc<SharedMockState>>,
    Path((name, hex)): Path<(String, String)>,
) -> Response {
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return (StatusCode::BAD_REQUEST, "invalid RRGGBB hex color").into_response();
    }
    let mut w = s.world.lock().expect("lock");
    match w.levers.iter_mut().find(|l| l.name == name) {
        Some(l) => {
            l.color = Some(hex.to_ascii_uppercase());
            (StatusCode::OK, "ok").into_response()
        }
        None => (StatusCode::NOT_FOUND, format!("lever '{name}' not found")).into_response(),
    }
}

async fn list_adapters(State(s): State<Arc<SharedMockState>>) -> Response {
    let w = s.world.lock().expect("lock");
    Json(w.adapters.clone()).into_response()
}

async fn get_adapter(State(s): State<Arc<SharedMockState>>, Path(name): Path<String>) -> Response {
    let w = s.world.lock().expect("lock");
    match w.adapters.iter().find(|a| a.name == name) {
        Some(a) => Json(a.clone()).into_response(),
        None => (StatusCode::NOT_FOUND, format!("adapter '{name}' not found")).into_response(),
    }
}
