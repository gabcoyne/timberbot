use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::http::AgentState;

/// Canonical WebSocket envelope from `docs/websocket-protocol.md`:
/// `{"type": "<name>", "payload": { ... }}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsEnvelope {
    #[serde(rename = "type")]
    pub frame_type: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameEventPayload {
    pub event: String,
    pub day: u32,
    pub timestamp: i64,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeartbeatPayload {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub agent_status: String,
    #[serde(default)]
    pub acked_request_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WsErrorPayload {
    pub error: String,
}

/// Typed representation of server -> client WebSocket messages.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerFrame {
    State(AgentState),
    Event(GameEventPayload),
    Error(WsErrorPayload),
    Pong,
    Unknown { frame_type: String, payload: Value },
}

impl ServerFrame {
    pub fn from_envelope(env: WsEnvelope) -> Result<Self, serde_json::Error> {
        match env.frame_type.to_ascii_lowercase().as_str() {
            "state" => Ok(Self::State(serde_json::from_value(env.payload)?)),
            "event" => Ok(Self::Event(serde_json::from_value(env.payload)?)),
            "error" => Ok(Self::Error(serde_json::from_value(env.payload)?)),
            "pong" => Ok(Self::Pong),
            _ => Ok(Self::Unknown {
                frame_type: env.frame_type,
                payload: env.payload,
            }),
        }
    }

    pub fn to_envelope(&self) -> Result<WsEnvelope, serde_json::Error> {
        match self {
            Self::State(state) => Ok(WsEnvelope {
                frame_type: "state".to_string(),
                payload: serde_json::to_value(state)?,
            }),
            Self::Event(evt) => Ok(WsEnvelope {
                frame_type: "event".to_string(),
                payload: serde_json::to_value(evt)?,
            }),
            Self::Error(err) => Ok(WsEnvelope {
                frame_type: "error".to_string(),
                payload: serde_json::to_value(err)?,
            }),
            Self::Pong => Ok(WsEnvelope {
                frame_type: "pong".to_string(),
                payload: serde_json::json!({}),
            }),
            Self::Unknown {
                frame_type,
                payload,
            } => Ok(WsEnvelope {
                frame_type: frame_type.clone(),
                payload: payload.clone(),
            }),
        }
    }
}

/// Typed representation of client -> server WebSocket messages.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientFrame {
    Heartbeat(HeartbeatPayload),
    Ping,
}

impl ClientFrame {
    pub fn to_envelope(&self) -> Result<WsEnvelope, serde_json::Error> {
        match self {
            Self::Heartbeat(hb) => Ok(WsEnvelope {
                frame_type: "heartbeat".to_string(),
                payload: serde_json::to_value(hb)?,
            }),
            Self::Ping => Ok(WsEnvelope {
                frame_type: "ping".to_string(),
                payload: serde_json::json!({}),
            }),
        }
    }
}
