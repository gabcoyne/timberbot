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
    use crate::http::{AgentMode, AgentState};
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
        use crate::http::*;

        let ping: PingResponse = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/ping.json"
        ))
        .unwrap();
        assert_eq!(ping.status, "ok");
        assert!(ping.ready);
        assert_eq!(ping.openapi_version, OPENAPI_VERSION);

        let settlement: SettlementName = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/settlement.json"
        ))
        .unwrap();
        assert!(!settlement.name.is_empty());

        let state: AgentState = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/agent_state.json"
        ))
        .unwrap();
        assert!(!state.ready);

        let summary: Summary = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/summary.json"
        ))
        .unwrap();
        assert_eq!(summary.faction, "Folktails");
        assert!(!summary.districts.is_empty());
        assert!(summary.trees.is_some());

        let time: TimeInfo = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/time.json"
        ))
        .unwrap();
        assert!(time.day_number >= 1);

        let weather: WeatherInfo = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/weather.json"
        ))
        .unwrap();
        assert_eq!(
            weather.cycle_length_in_days,
            weather.temperate_weather_duration + weather.hazardous_weather_duration
        );

        let speed: SpeedInfo = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/speed.json"
        ))
        .unwrap();
        assert!(speed.speed <= 3);

        let wh: WorkHours = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/workhours.json"
        ))
        .unwrap();
        assert!(wh.end_hours >= 1.0);

        let alerts: Vec<AlertItem> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/alerts.json"
        ))
        .unwrap();
        assert!(alerts.is_empty());

        let notifications: Vec<NotificationItem> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/notifications.json"
        ))
        .unwrap();
        assert!(!notifications.is_empty());

        let buildings: PaginatedList<Building> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/buildings.json"
        ))
        .unwrap();
        assert_eq!(buildings.total, 1);
        assert_eq!(buildings.items[0].name, "DistrictCenter.Folktails");

        let beavers: PaginatedList<Beaver> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/beavers.json"
        ))
        .unwrap();
        assert_eq!(beavers.total, 13);
        assert_eq!(beavers.items[0].lifting_capacity, Some(14));

        let trees: PaginatedList<Plant> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/trees.json"
        ))
        .unwrap();
        assert!(trees.total > 0);

        let crops: PaginatedList<Plant> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/crops.json"
        ))
        .unwrap();
        assert_eq!(crops.limit, 100);
        assert_eq!(crops.total, 1857);

        let gatherables: PaginatedList<Gatherable> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/gatherables.json"
        ))
        .unwrap();
        assert!(gatherables.total > 0);

        let tree_clusters: Vec<Cluster> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/tree_clusters.json"
        ))
        .unwrap();
        assert!(!tree_clusters.is_empty());

        let food_clusters: Vec<Cluster> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/food_clusters.json"
        ))
        .unwrap();
        assert!(!food_clusters.is_empty());

        let pop: Vec<PopulationItem> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/population.json"
        ))
        .unwrap();
        assert_eq!(pop[0].adults, 10);

        let resources: ResourcesMap = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/resources.json"
        ))
        .unwrap();
        assert!(!resources.is_empty());

        let districts: Vec<District> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/districts.json"
        ))
        .unwrap();
        assert!(!districts.is_empty());

        let distribution: Vec<DistrictDistribution> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/distribution.json"
        ))
        .unwrap();
        assert!(!distribution.is_empty());

        let power: Vec<PowerNetwork> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/power.json"
        ))
        .unwrap();
        assert!(!power.is_empty());

        let prefabs: Vec<PrefabInfo> = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/prefabs.json"
        ))
        .unwrap();
        assert_eq!(prefabs.len(), 178);
        let farm = prefabs
            .iter()
            .find(|p| p.name == "EfficientFarmHouse.Folktails")
            .expect("EfficientFarmHouse.Folktails in recorded fixture");
        assert!(farm.unlocked);
        assert_eq!(farm.cost[0].amount, 25);

        let science: ScienceInfo = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/science.json"
        ))
        .unwrap();
        assert!(!science.unlockables.is_empty());

        let tiles: TilesResponse = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/tiles.json"
        ))
        .unwrap();
        assert_eq!(tiles.map_size.z, 23);

        let wellbeing: WellbeingReport = serde_json::from_str(include_str!(
            "../../../../python/tests/fixtures/openapi/wellbeing.json"
        ))
        .unwrap();
        let max_wb: f64 = wellbeing.categories.iter().map(|c| c.max).sum();
        assert!((max_wb - 78.0).abs() < 1e-6);
    }
}
