use std::path::PathBuf;
use std::time::Duration;

use serde_json::{json, Value};
use timberbot_client::{
    ClientConfig, ClientError, LeverClient, TimberbotClient, TimberbotWsConnection,
};
use timberbot_mcp::{McpServer, McpServerConfig};
use timberbot_mock::sim::ScenarioConfig;
use timberbot_mock::{MockServerConfig, RunningMockServer};
use timberbot_protocol::ws::{HeartbeatPayload, ServerFrame};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("workspace crate is under repo/rust/crates/<crate>")
        .to_path_buf()
}

fn mock_cfg(auth_token: &str, initial_ready: bool) -> MockServerConfig {
    MockServerConfig {
        listen_address: "127.0.0.1".to_string(),
        http_port: 0,
        ws_port: 0,
        lever_port: Some(0),
        auth_token: auth_token.to_string(),
        initial_ready,
        tick_interval: None,
        days_per_tick_speed1: 0.25,
    }
}

#[tokio::test]
async fn non_loopback_without_auth_token_refuses_to_start() {
    let cfg = MockServerConfig {
        listen_address: "0.0.0.0".to_string(),
        http_port: 0,
        ws_port: 0,
        lever_port: None,
        auth_token: String::new(),
        initial_ready: false,
        tick_interval: None,
        days_per_tick_speed1: 0.1,
    };
    let err = RunningMockServer::start(cfg, ScenarioConfig::default())
        .await
        .err()
        .expect("non-loopback bind without authToken must fail");
    assert!(
        err.to_string().contains("non-loopback"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn http_auth_ready_gate_and_ws_protocol_contract() {
    let token = "test-secret-token";
    let server = RunningMockServer::start(mock_cfg(token, false), ScenarioConfig::default())
        .await
        .expect("start mock server");

    // 1. Unauthenticated client: /api/ping succeeds, /api/agent/state & /api/summary return 401
    let unauth = TimberbotClient::new(ClientConfig {
        host: "127.0.0.1".to_string(),
        http_port: server.http_addr.port(),
        ws_port: server.ws_addr.port(),
        auth_token: None,
    })
    .unwrap();

    let ping = unauth.ping().await.expect("/api/ping is unauthenticated");
    assert_eq!(ping.status, "ok");

    assert!(matches!(
        unauth.agent_state().await,
        Err(ClientError::Unauthorized { .. })
    ));
    assert!(matches!(
        unauth.summary().await,
        Err(ClientError::Unauthorized { .. })
    ));

    // 2. Authenticated client while ready=false: /api/agent/state works, /api/summary returns 409
    let client = TimberbotClient::new(ClientConfig {
        host: "127.0.0.1".to_string(),
        http_port: server.http_addr.port(),
        ws_port: server.ws_addr.port(),
        auth_token: Some(token.to_string()),
    })
    .unwrap();

    let st = client
        .agent_state()
        .await
        .expect("agent_state is gate-exempt");
    assert!(!st.ready);
    assert!(matches!(
        client.summary().await,
        Err(ClientError::GameNotReady { .. })
    ));

    // 3. WebSocket works even while ready=false, pushes initial state, acks heartbeat, broadcasts events
    let ws_url = format!("ws://127.0.0.1:{}/api/ws", server.ws_addr.port());
    let mut ws = TimberbotWsConnection::connect(&ws_url, Some(token))
        .await
        .expect("ws connect with bearer auth");

    let first = tokio::time::timeout(Duration::from_secs(2), ws.next_frame())
        .await
        .expect("initial state timeout")
        .expect("ws frame ok")
        .expect("ws open");
    match first {
        ServerFrame::State(s) => assert!(!s.ready),
        other => panic!("expected initial state frame, got {other:?}"),
    }

    // Open ready gate -> WS receives updated state frame, HTTP /api/summary succeeds
    client.set_ready(true).await.expect("set_ready(true)");
    let state_after_ready = tokio::time::timeout(Duration::from_secs(2), ws.next_frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match state_after_ready {
        ServerFrame::State(s) => assert!(s.ready),
        other => panic!("expected ready state frame, got {other:?}"),
    }

    let summary = client
        .summary()
        .await
        .expect("summary works when ready=true");
    assert_eq!(summary.faction, "Folktails");

    // Enqueue agent request -> WS pushes pendingRequest -> heartbeat ack clears it
    client
        .submit_agent_request("check water")
        .await
        .expect("submit_agent_request");
    let pending_frame = tokio::time::timeout(Duration::from_secs(2), ws.next_frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let req_id = match pending_frame {
        ServerFrame::State(s) => s.pending_request.expect("pending_request set").id,
        other => panic!("expected state frame with pendingRequest, got {other:?}"),
    };

    ws.send_heartbeat(HeartbeatPayload {
        version: "0.1.0".to_string(),
        agent_status: "idle".to_string(),
        acked_request_id: req_id,
    })
    .await
    .expect("send heartbeat");

    let cleared_frame = tokio::time::timeout(Duration::from_secs(2), ws.next_frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    match cleared_frame {
        ServerFrame::State(s) => assert!(s.pending_request.is_none()),
        other => panic!("expected cleared state frame, got {other:?}"),
    }

    // Step simulation 1 full day -> receive day.start event frame
    server.state.step_sim(1.0);
    let mut saw_day_start = false;
    for _ in 0..4 {
        if let Ok(Ok(Some(ServerFrame::Event(ev)))) =
            tokio::time::timeout(Duration::from_millis(400), ws.next_frame()).await
        {
            if ev.event == "day.start" {
                saw_day_start = true;
                break;
            }
        }
    }
    assert!(saw_day_start, "expected day.start WS event frame");

    ws.close().await.ok();
    server.shutdown();
}

#[tokio::test]
async fn mcp_server_exercises_mod_lever_and_save_tools() {
    let server = RunningMockServer::start(mock_cfg("", true), ScenarioConfig::default())
        .await
        .expect("start mock server");
    let lever_port = server.lever_addr.expect("lever server running").port();
    let scenarios_dir = repo_root().join("scenarios");

    let mcp = McpServer::new(McpServerConfig {
        mod_client: ClientConfig {
            host: "127.0.0.1".to_string(),
            http_port: server.http_addr.port(),
            ws_port: server.ws_addr.port(),
            auth_token: None,
        },
        lever_base_url: format!("http://127.0.0.1:{lever_port}"),
        allowed_save_root: Some(scenarios_dir.clone()),
    })
    .expect("create McpServer");

    // 1. initialize
    let init_resp: Value = serde_json::from_str(
        &mcp.handle_json_line(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}).to_string(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(init_resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "timberbot-mcp");

    // 2. tools/list
    let list_resp: Value = serde_json::from_str(
        &mcp.handle_json_line(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string())
            .await
            .unwrap(),
    )
    .unwrap();
    let tools = list_resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 35);

    // 3. Mod read/write via MCP
    let summary_resp: Value = serde_json::from_str(
        &mcp.handle_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "timberbot_summary", "arguments": {}}
            })
            .to_string(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(summary_resp["result"]["isError"], false);
    let summary_text = summary_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let summary_json: Value = serde_json::from_str(summary_text).unwrap();
    assert_eq!(summary_json["settlement"], "MockColony");

    // 4. Official :8080 HTTP Lever toggle via MCP + verified via LeverClient
    let on_resp: Value = serde_json::from_str(
        &mcp.handle_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {"name": "lever_switch_on", "arguments": {"name": "MainGate"}}
            })
            .to_string(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(on_resp["result"]["isError"], false);

    let lever_client = LeverClient::new(format!("http://127.0.0.1:{lever_port}")).unwrap();
    let lever = lever_client.get_lever("MainGate").await.unwrap();
    assert!(lever.state, "MainGate lever should be ON");

    // 5. Read-only .timber save inspection via MCP
    let save_path = scenarios_dir.join("sample.timber");
    let save_resp: Value = serde_json::from_str(
        &mcp.handle_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {"name": "save_inspect", "arguments": {"path": save_path.display().to_string()}}
            })
            .to_string(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(save_resp["result"]["isError"], false);
    let save_json: Value =
        serde_json::from_str(save_resp["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(save_json["game_version"], "1.1.2.0");
    assert_eq!(save_json["day_number"], 10);
    assert_eq!(save_json["stock_totals"]["Water"], 90);

    // 6. Save root confinement blocks paths outside allowed_save_root
    let escape_resp: Value = serde_json::from_str(
        &mcp.handle_json_line(
            &json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": {"name": "save_inspect", "arguments": {"path": "/etc/hosts"}}
            })
            .to_string(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(escape_resp["result"]["isError"], true);

    server.shutdown();
}
