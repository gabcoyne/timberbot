use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use timberbot_protocol::ws::{HeartbeatPayload, ServerFrame, WsEnvelope, WsErrorPayload};

use crate::http_server::{extract_bearer, token_matches};
use crate::SharedMockState;

pub fn build_router(state: Arc<SharedMockState>) -> Router {
    Router::new()
        .route("/api/ws", get(ws_upgrade))
        .with_state(state)
}

async fn ws_upgrade(
    State(state): State<Arc<SharedMockState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Response {
    if !state.auth_token.is_empty() {
        let auth_hdr = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
        let presented = extract_bearer(auth_hdr)
            .map(ToOwned::to_owned)
            .or_else(|| query.get("token").cloned());
        if !presented
            .as_deref()
            .is_some_and(|tok| token_matches(&state.auth_token, tok))
        {
            let mut resp = (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized"})),
            )
                .into_response();
            resp.headers_mut().insert(
                WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer realm=\"timberbot\""),
            );
            return resp;
        }
    }

    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: Arc<SharedMockState>, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();

    // Immediately push current AgentState snapshot per docs/websocket-protocol.md.
    let initial = {
        let w = state.world.lock().expect("lock");
        ServerFrame::State(w.agent_state.clone())
    };
    if send_frame(&mut sender, &initial).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            broadcast_res = rx.recv() => {
                match broadcast_res {
                    Ok(frame) => {
                        if send_frame(&mut sender, &frame).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            inbound = receiver.next() => {
                let Some(Ok(msg)) = inbound else {
                    break;
                };
                match msg {
                    Message::Text(txt) => {
                        let reply = handle_inbound_text(&state, &txt);
                        if let Some(frame) = reply {
                            if send_frame(&mut sender, &frame).await.is_err() {
                                break;
                            }
                        }
                    }
                    Message::Close(_) => break,
                    Message::Ping(bytes) => {
                        if sender.send(Message::Pong(bytes)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn handle_inbound_text(state: &Arc<SharedMockState>, txt: &str) -> Option<ServerFrame> {
    let env: WsEnvelope = match serde_json::from_str(txt) {
        Ok(e) => e,
        Err(_) => {
            return Some(ServerFrame::Error(WsErrorPayload {
                error: "invalid_message".to_string(),
            }));
        }
    };

    match env.frame_type.to_ascii_lowercase().as_str() {
        "ping" => Some(ServerFrame::Pong),
        "heartbeat" => {
            let hb: HeartbeatPayload =
                serde_json::from_value(env.payload).unwrap_or(HeartbeatPayload {
                    version: String::new(),
                    agent_status: String::new(),
                    acked_request_id: 0,
                });
            let snap = {
                let mut w = state.world.lock().expect("lock");
                w.apply_heartbeat(&hb.agent_status, hb.acked_request_id);
                w.agent_state.clone()
            };
            Some(ServerFrame::State(snap))
        }
        other => Some(ServerFrame::Error(WsErrorPayload {
            error: format!("unknown_type: {other}"),
        })),
    }
}

async fn send_frame(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    frame: &ServerFrame,
) -> Result<(), axum::Error> {
    if let Ok(env) = frame.to_envelope() {
        if let Ok(txt) = serde_json::to_string(&env) {
            sender.send(Message::Text(txt)).await?;
        }
    }
    Ok(())
}
