pub mod http;
pub mod lever;
pub mod save;
pub mod ws;

/// Matches `TimberbotPure.OPENAPI_VERSION` in `timberbot/src/TimberbotPure.cs`.
pub const OPENAPI_VERSION: &str = "1.0.0";

/// Matches `TimberbotAgentState.GameNotReadyJson` in `timberbot/src/TimberbotAgentState.cs`.
pub const GAME_NOT_READY_ERROR: &str = "game_not_ready";
pub const GAME_NOT_READY_HINT: &str = "player must press Launch in the Timberbot widget";

/// Default ports documented in `timberbot/src/settings.json` and the official HTTP Lever docs.
pub const DEFAULT_HTTP_PORT: u16 = 8085;
pub const DEFAULT_WS_PORT: u16 = 8086;
pub const DEFAULT_LEVER_PORT: u16 = 8080;

/// Returns true if a path is exempt from the ready gate, mirroring
/// `TimberbotAgentState.IsGateExempt` in `timberbot/src/TimberbotAgentState.cs`.
pub fn is_gate_exempt(path: &str) -> bool {
    let trimmed = path.trim_end_matches('/').to_ascii_lowercase();
    trimmed == "/api/ping"
        || trimmed == "/api/ready"
        || trimmed == "/api/agent"
        || trimmed.starts_with("/api/agent/")
}

/// Returns true if `listen_address` is a loopback alias, mirroring
/// `TimberbotPure.IsLoopbackAddress`.
pub fn is_loopback_address(listen_address: &str) -> bool {
    let addr = listen_address.trim().to_ascii_lowercase();
    addr.is_empty() || addr == "localhost" || addr == "127.0.0.1" || addr == "::1"
}

/// Returns true if the server must refuse to start, mirroring
/// `TimberbotPure.RequiresAuthToken`.
pub fn requires_auth_token(listen_address: &str, auth_token: &str) -> bool {
    if is_loopback_address(listen_address) {
        return false;
    }
    auth_token.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{AgentMode, AgentState, PingResponse, Summary};
    use crate::ws::{ClientFrame, HeartbeatPayload, ServerFrame, WsEnvelope};

    #[test]
    fn gate_exempt_matches_csharp_whitelist() {
        assert!(is_gate_exempt("/api/ping"));
        assert!(is_gate_exempt("/api/ping/"));
        assert!(is_gate_exempt("/api/ready"));
        assert!(is_gate_exempt("/api/agent/state"));
        assert!(is_gate_exempt("/api/agent/config"));
        assert!(is_gate_exempt("/api/agent/request"));
        assert!(is_gate_exempt("/api/agent/message"));
        assert!(!is_gate_exempt("/api/summary"));
        assert!(!is_gate_exempt("/api/buildings"));
        assert!(!is_gate_exempt("/api/speed"));
    }

    #[test]
    fn non_loopback_requires_auth_token() {
        assert!(!requires_auth_token("127.0.0.1", ""));
        assert!(!requires_auth_token("localhost", ""));
        assert!(!requires_auth_token("::1", ""));
        assert!(requires_auth_token("0.0.0.0", ""));
        assert!(requires_auth_token("+", "   "));
        assert!(!requires_auth_token("0.0.0.0", "s3cret"));
    }

    #[test]
    fn ws_envelope_roundtrip() {
        let state = AgentState {
            mode: AgentMode::Request,
            goal: "survive drought".to_string(),
            ready: true,
            pending_request: None,
            agent_status: "idle".to_string(),
            last_error: None,
        };
        let env = ServerFrame::State(state.clone()).to_envelope().unwrap();
        assert_eq!(env.frame_type, "state");
        let parsed = ServerFrame::from_envelope(env).unwrap();
        assert_eq!(parsed, ServerFrame::State(state));

        let hb_env = ClientFrame::Heartbeat(HeartbeatPayload {
            version: "0.1.0".to_string(),
            agent_status: "running".to_string(),
            acked_request_id: 7,
        })
        .to_envelope()
        .unwrap();
        assert_eq!(hb_env.frame_type, "heartbeat");
        assert_eq!(hb_env.payload["acked_request_id"], 7);

        let unknown = ServerFrame::from_envelope(WsEnvelope {
            frame_type: "future_frame".to_string(),
            payload: serde_json::json!({"k": 1}),
        })
        .unwrap();
        assert!(matches!(unknown, ServerFrame::Unknown { .. }));
    }

    #[test]
    fn golden_openapi_fixtures_deserialize() {
        let ping_raw = include_str!("../../../../python/tests/fixtures/openapi/ping.json");
        let ping: PingResponse = serde_json::from_str(ping_raw).unwrap();
        assert_eq!(ping.status, "ok");
        assert_eq!(ping.openapi_version, OPENAPI_VERSION);

        let summary_raw = include_str!("../../../../python/tests/fixtures/openapi/summary.json");
        let summary: Summary = serde_json::from_str(summary_raw).unwrap();
        assert_eq!(summary.faction, "Folktails");
        assert!(!summary.districts.is_empty());

        let state_raw = include_str!("../../../../python/tests/fixtures/openapi/agent_state.json");
        let state: AgentState = serde_json::from_str(state_raw).unwrap();
        assert!(!state.ready);
    }
}
