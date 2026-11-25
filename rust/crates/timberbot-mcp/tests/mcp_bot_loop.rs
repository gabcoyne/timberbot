use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use timberbot_agent::managers::{default_orientation, plan_actions, Action, Observation};
use timberbot_client::ClientConfig;
use timberbot_mcp::{McpServer, McpServerConfig};
use timberbot_mock::data::Difficulty;
use timberbot_mock::sim::ScenarioConfig;
use timberbot_mock::{MockServerConfig, RunningMockServer};
use timberbot_protocol::http::{
    Building, FindPlacementResponse, PaginatedList, Summary, TilesResponse, WorkHours,
};
use timberbot_protocol::lever::HttpLeverState;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("crate sits under repo/rust/crates/<crate>")
        .to_path_buf()
}

/// Call a single MCP tool via JSON-RPC 2.0 `tools/call` and parse its text payload as JSON.
async fn mcp_call_json<T: DeserializeOwned>(
    mcp: &McpServer,
    req_id: &mut u64,
    tool: &str,
    args: Value,
) -> T {
    *req_id += 1;
    let payload = json!({
        "jsonrpc": "2.0",
        "id": *req_id,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": args
        }
    });
    let raw = mcp
        .handle_json_line(&payload.to_string())
        .await
        .expect("MCP JSON-RPC response");
    let resp: Value = serde_json::from_str(&raw).expect("valid JSON-RPC envelope");
    assert_eq!(
        resp["result"]["isError"],
        json!(false),
        "MCP tool `{tool}` failed: {resp}"
    );
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("MCP text content");
    serde_json::from_str(text).expect("tool result JSON deserialization")
}

async fn mcp_call_ok(mcp: &McpServer, req_id: &mut u64, tool: &str, args: Value) -> Value {
    *req_id += 1;
    let payload = json!({
        "jsonrpc": "2.0",
        "id": *req_id,
        "method": "tools/call",
        "params": {
            "name": tool,
            "arguments": args
        }
    });
    let raw = mcp
        .handle_json_line(&payload.to_string())
        .await
        .expect("MCP JSON-RPC response");
    let resp: Value = serde_json::from_str(&raw).expect("valid JSON-RPC envelope");
    assert_eq!(
        resp["result"]["isError"],
        json!(false),
        "MCP tool `{tool}` failed: {resp}"
    );
    resp
}

/// End-to-end local Grok Bot loop driven 100% through `timberbot-mcp` JSON-RPC 2.0 `tools/call`.
/// Exercises:
/// - `initialize` + `tools/list`
/// - Offline `.timber` inspection of the real `scenarios/practice-valley.timber` map
/// - Full pause -> observe -> plan -> act -> unpause survival across multiple droughts AND a badtide
///   for both Folktails and Iron Teeth on Normal difficulty
/// - Official `:8080` HTTP Lever switching (`lever_switch_on` / `lever_switch_off`) on hazards
async fn run_mcp_bot_survival(faction: &str, target_cycles: u32) {
    let mut scenario = ScenarioConfig::preset(faction, Difficulty::Normal);
    scenario.levers.push(HttpLeverState {
        name: "HazardAlarm".to_string(),
        state: false,
        spring_return: false,
        color: Some("FF6600".to_string()),
    });
    let initial_pop = scenario.adults + scenario.children + scenario.bots;

    let server = RunningMockServer::start(
        MockServerConfig {
            listen_address: "127.0.0.1".to_string(),
            http_port: 0,
            ws_port: 0,
            lever_port: Some(0),
            auth_token: "bot-mcp-token".to_string(),
            initial_ready: true,
            tick_interval: None,
            days_per_tick_speed1: 0.25,
            ..Default::default()
        },
        scenario,
    )
    .await
    .expect("start mock server");

    let lever_port = server.lever_addr.expect("lever server").port();
    let scenarios_dir = repo_root().join("scenarios");

    let mcp = McpServer::new(McpServerConfig {
        mod_client: ClientConfig {
            host: "127.0.0.1".to_string(),
            http_port: server.http_addr.port(),
            ws_port: server.ws_addr.port(),
            auth_token: Some("bot-mcp-token".to_string()),
        },
        lever_base_url: format!("http://127.0.0.1:{lever_port}"),
        allowed_save_root: Some(scenarios_dir.clone()),
    })
    .expect("create McpServer");

    let mut req_id = 0u64;

    // 1. Handshake + verify real .timber inspection via MCP
    let init_raw = mcp
        .handle_json_line(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}).to_string(),
        )
        .await
        .unwrap();
    let init_val: Value = serde_json::from_str(&init_raw).unwrap();
    assert_eq!(init_val["result"]["serverInfo"]["name"], "timberbot-mcp");

    let pv_path = scenarios_dir.join("practice-valley.timber");
    let pv_report: Value = mcp_call_json(
        &mcp,
        &mut req_id,
        "save_inspect",
        json!({"path": pv_path.display().to_string()}),
    )
    .await;
    assert_eq!(pv_report["entity_count"], 1644);
    assert_eq!(pv_report["map_size"]["x"], 128);

    // 2. Local-bot loop exclusively through MCP tools/call
    let mut lever_is_on = false;
    let mut mcp_tool_calls = 0usize;

    for _ in 0..(target_cycles as usize * 80) {
        // PAUSE
        mcp_call_ok(
            &mcp,
            &mut req_id,
            "timberbot_set_speed",
            json!({"speed": 0}),
        )
        .await;
        mcp_tool_calls += 1;

        // OBSERVE via MCP
        let summary: Summary =
            mcp_call_json(&mcp, &mut req_id, "timberbot_summary", json!({})).await;
        let bldgs: PaginatedList<Building> = mcp_call_json(
            &mcp,
            &mut req_id,
            "timberbot_buildings",
            json!({"limit": 0}),
        )
        .await;
        let tiles: TilesResponse = mcp_call_json(
            &mcp,
            &mut req_id,
            "timberbot_tiles",
            json!({"x1": 8, "y1": 10, "x2": 14, "y2": 18}),
        )
        .await;
        let wh: WorkHours =
            mcp_call_json(&mcp, &mut req_id, "timberbot_workhours", json!({})).await;
        mcp_tool_calls += 4;

        let obs = Observation {
            summary,
            buildings: bldgs.items,
            tiles,
            work_end_hours: wh.end_hours.round().clamp(1.0, 24.0) as u8,
        };

        // Toggle official :8080 HTTP Lever to mirror hazard alarm state
        let want_lever = obs.summary.weather.is_hazardous || obs.days_until_hazard() <= 3;
        if want_lever != lever_is_on {
            let tool = if want_lever {
                "lever_switch_on"
            } else {
                "lever_switch_off"
            };
            mcp_call_ok(&mcp, &mut req_id, tool, json!({"name": "HazardAlarm"})).await;
            lever_is_on = want_lever;
            mcp_tool_calls += 1;
        }

        // PLAN using the deterministic managers & ACT sequentially through MCP tools
        let actions = plan_actions(&obs);
        for action in actions {
            match action {
                Action::SetFloodgate { id, height, .. } => {
                    mcp_call_ok(
                        &mcp,
                        &mut req_id,
                        "timberbot_set_floodgate",
                        json!({"id": id, "height": height}),
                    )
                    .await;
                }
                Action::PauseBuilding { id, paused, .. } => {
                    mcp_call_ok(
                        &mcp,
                        &mut req_id,
                        "timberbot_pause_building",
                        json!({"id": id, "paused": paused}),
                    )
                    .await;
                }
                Action::SetPriority { id, priority, .. } => {
                    mcp_call_ok(
                        &mcp,
                        &mut req_id,
                        "timberbot_set_priority",
                        json!({"id": id, "priority": priority, "type": "workplace"}),
                    )
                    .await;
                }
                Action::SetWorkers { id, count, .. } => {
                    mcp_call_ok(
                        &mcp,
                        &mut req_id,
                        "timberbot_set_workers",
                        json!({"id": id, "count": count}),
                    )
                    .await;
                }
                Action::SetWorkHours { end_hours, .. } => {
                    mcp_call_ok(
                        &mcp,
                        &mut req_id,
                        "timberbot_set_workhours",
                        json!({"endHours": end_hours}),
                    )
                    .await;
                }
                Action::FindAndPlace { prefab, .. } => {
                    let found: FindPlacementResponse = mcp_call_json(
                        &mcp,
                        &mut req_id,
                        "timberbot_find_placement",
                        json!({"prefab": prefab, "x": 16, "y": 16, "radius": 10}),
                    )
                    .await;
                    mcp_tool_calls += 1;
                    if let Some(c) = found.candidates.first() {
                        mcp_call_ok(
                            &mcp,
                            &mut req_id,
                            "timberbot_place_building",
                            json!({
                                "prefab": prefab,
                                "x": c.x,
                                "y": c.y,
                                "z": c.z,
                                "orientation": default_orientation()
                            }),
                        )
                        .await;
                    }
                }
                Action::PlantCrops {
                    x1,
                    y1,
                    x2,
                    y2,
                    z,
                    crop,
                    ..
                } => {
                    mcp_call_ok(
                        &mcp,
                        &mut req_id,
                        "timberbot_plant_crop",
                        json!({"x1": x1, "y1": y1, "x2": x2, "y2": y2, "z": z, "crop": crop}),
                    )
                    .await;
                }
                Action::MarkTrees {
                    x1, y1, x2, y2, z, ..
                } => {
                    mcp_call_ok(
                        &mcp,
                        &mut req_id,
                        "timberbot_mark_trees",
                        json!({"x1": x1, "y1": y1, "x2": x2, "y2": y2, "z": z, "marked": true}),
                    )
                    .await;
                }
            }
            mcp_tool_calls += 1;
        }

        // UNPAUSE
        mcp_call_ok(
            &mcp,
            &mut req_id,
            "timberbot_set_speed",
            json!({"speed": 3}),
        )
        .await;
        mcp_tool_calls += 1;

        // Advance half a simulated day
        server.state.step_sim(0.5 / 3.0);

        let completed = server
            .state
            .world
            .lock()
            .expect("lock")
            .metrics
            .cycles_completed;
        if completed >= target_cycles {
            break;
        }
    }

    let metrics = server.state.world.lock().expect("lock").metrics.clone();
    let final_summary: Summary =
        mcp_call_json(&mcp, &mut req_id, "timberbot_summary", json!({})).await;
    let final_pop = final_summary.districts[0].population.total_beavers();

    eprintln!(
        "MCP Bot Loop ({faction} Normal): tool_calls={mcp_tool_calls}, cycles={}, droughts={}, badtides={}, pop={initial_pop}->{final_pop}, deaths={}, starv={}, thirst={}, contam={}",
        metrics.cycles_completed,
        metrics.droughts_survived,
        metrics.badtides_survived,
        metrics.deaths,
        metrics.starvation_events,
        metrics.thirst_events,
        metrics.contamination_events
    );

    assert!(metrics.cycles_completed >= target_cycles);
    assert!(metrics.droughts_survived >= 3);
    assert!(metrics.badtides_survived >= 1);
    assert_eq!(metrics.deaths, 0);
    assert_eq!(metrics.starvation_events, 0);
    assert_eq!(metrics.thirst_events, 0);
    assert_eq!(metrics.contamination_events, 0);
    assert!(final_pop >= initial_pop);
    assert!(mcp_tool_calls > 100);

    server.shutdown();
}

#[tokio::test]
async fn mcp_bot_loop_end_to_end_folktails_and_iron_teeth() {
    run_mcp_bot_survival("Folktails", 5).await;
    run_mcp_bot_survival("IronTeeth", 5).await;
}
