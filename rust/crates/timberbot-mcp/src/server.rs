use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use timberbot_client::{ClientConfig, LeverClient, TimberbotClient};
use timberbot_protocol::http::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub mod_client: ClientConfig,
    pub lever_base_url: String,
    pub allowed_save_root: Option<PathBuf>,
}

pub struct McpServer {
    mod_client: TimberbotClient,
    lever_client: LeverClient,
    allowed_save_root: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl McpServer {
    pub fn new(config: McpServerConfig) -> Result<Self> {
        Ok(Self {
            mod_client: TimberbotClient::new(config.mod_client)?,
            lever_client: LeverClient::new(config.lever_base_url)?,
            allowed_save_root: config.allowed_save_root,
        })
    }

    pub fn tool_definitions(&self) -> Vec<Value> {
        vec![
            // --- Timberbot mod (:8085) read tools ---
            tool_def("timberbot_ping", "Probe mod liveness and OpenAPI version (unauthenticated, gate-exempt).", json!({"type": "object", "properties": {}})),
            tool_def("timberbot_agent_state", "Read widget/connector state (mode, goal, ready, pendingRequest).", json!({"type": "object", "properties": {}})),
            tool_def("timberbot_summary", "Fetch compact colony snapshot (time, weather, districts, stocks, housing, employment, alerts).", json!({"type": "object", "properties": {}})),
            tool_def("timberbot_weather", "Fetch current cycle, cycleDay, temperate/hazardous duration, and isHazardous.", json!({"type": "object", "properties": {}})),
            tool_def("timberbot_resources", "Fetch per-district resource stocks (available and all).", json!({"type": "object", "properties": {}})),
            tool_def("timberbot_buildings", "List buildings with optional name filter, id lookup, or pagination.", json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "id": {"type": "integer"},
                    "limit": {"type": "integer"},
                    "offset": {"type": "integer"}
                }
            })),
            tool_def("timberbot_beavers", "List beavers and bots with wellbeing and critical need flags.", json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "limit": {"type": "integer"},
                    "offset": {"type": "integer"}
                }
            })),
            tool_def("timberbot_tiles", "Inspect map size or tile rect (terrainHeight, waterDepth, contamination, moist, occupant).", json!({
                "type": "object",
                "properties": {
                    "x1": {"type": "integer"},
                    "y1": {"type": "integer"},
                    "x2": {"type": "integer"},
                    "y2": {"type": "integer"}
                }
            })),
            tool_def("timberbot_alerts", "List active colony alerts (unstaffed, unpowered, unreachable, status).", json!({"type": "object", "properties": {}})),
            tool_def("timberbot_science", "List science points and unlockable buildings.", json!({"type": "object", "properties": {}})),
            tool_def("timberbot_prefabs", "List buildable prefabs with footprints, science cost, and material costs.", json!({"type": "object", "properties": {}})),
            // --- Timberbot mod (:8085) write tools ---
            tool_def("timberbot_set_speed", "Set game speed (0=pause, 1=normal, 2=fast, 3=fastest).", json!({
                "type": "object",
                "required": ["speed"],
                "properties": {"speed": {"type": "integer", "minimum": 0, "maximum": 3}}
            })),
            tool_def("timberbot_set_workhours", "Set work end hour (1..24).", json!({
                "type": "object",
                "required": ["endHours"],
                "properties": {"endHours": {"type": "integer", "minimum": 1, "maximum": 24}}
            })),
            tool_def("timberbot_pause_building", "Pause or unpause a building by id.", json!({
                "type": "object",
                "required": ["id", "paused"],
                "properties": {"id": {"type": "integer"}, "paused": {"type": "boolean"}}
            })),
            tool_def("timberbot_set_workers", "Set desired worker count on a workplace building.", json!({
                "type": "object",
                "required": ["id", "count"],
                "properties": {"id": {"type": "integer"}, "count": {"type": "integer", "minimum": 0}}
            })),
            tool_def("timberbot_set_priority", "Set construction or workplace priority (VeryLow, Low, Normal, High, VeryHigh).", json!({
                "type": "object",
                "required": ["id", "priority"],
                "properties": {
                    "id": {"type": "integer"},
                    "priority": {"type": "string", "enum": ["VeryLow", "Low", "Normal", "High", "VeryHigh"]},
                    "type": {"type": "string", "enum": ["", "workplace", "construction"]}
                }
            })),
            tool_def("timberbot_set_floodgate", "Set floodgate water height (clamped 0..maxHeight).", json!({
                "type": "object",
                "required": ["id", "height"],
                "properties": {"id": {"type": "integer"}, "height": {"type": "number"}}
            })),
            tool_def("timberbot_set_storage", "Configure storage allowed good and mode (accept/obtain/supply/empty).", json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": {"type": "integer"},
                    "good": {"type": "string"},
                    "mode": {"type": "string", "enum": ["accept", "obtain", "supply", "empty"]}
                }
            })),
            tool_def("timberbot_find_placement", "Find valid building placement candidates near x,y or within x1..x2,y1..y2.", json!({
                "type": "object",
                "required": ["prefab"],
                "properties": {
                    "prefab": {"type": "string"},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"},
                    "radius": {"type": "integer"},
                    "x1": {"type": "integer"},
                    "y1": {"type": "integer"},
                    "x2": {"type": "integer"},
                    "y2": {"type": "integer"}
                }
            })),
            tool_def("timberbot_place_building", "Place a building prefab at (x, y, z) with orientation.", json!({
                "type": "object",
                "required": ["prefab", "x", "y", "z"],
                "properties": {
                    "prefab": {"type": "string"},
                    "x": {"type": "integer"},
                    "y": {"type": "integer"},
                    "z": {"type": "integer"},
                    "orientation": {"type": "string", "enum": ["south", "west", "north", "east"]}
                }
            })),
            tool_def("timberbot_place_path", "Place a path between (x1,y1) and (x2,y2).", json!({
                "type": "object",
                "required": ["x1", "y1", "x2", "y2"],
                "properties": {
                    "x1": {"type": "integer"},
                    "y1": {"type": "integer"},
                    "x2": {"type": "integer"},
                    "y2": {"type": "integer"},
                    "style": {"type": "string", "enum": ["direct", "straight"]}
                }
            })),
            tool_def("timberbot_demolish_building", "Demolish a building by id (destructive).", json!({
                "type": "object",
                "required": ["id"],
                "properties": {"id": {"type": "integer"}}
            })),
            tool_def("timberbot_plant_crop", "Mark a rectangle for planting a crop species.", json!({
                "type": "object",
                "required": ["x1", "y1", "x2", "y2", "z", "crop"],
                "properties": {
                    "x1": {"type": "integer"}, "y1": {"type": "integer"},
                    "x2": {"type": "integer"}, "y2": {"type": "integer"},
                    "z": {"type": "integer"}, "crop": {"type": "string"}
                }
            })),
            tool_def("timberbot_mark_trees", "Mark or unmark a rectangle of trees for cutting.", json!({
                "type": "object",
                "required": ["x1", "y1", "x2", "y2", "z"],
                "properties": {
                    "x1": {"type": "integer"}, "y1": {"type": "integer"},
                    "x2": {"type": "integer"}, "y2": {"type": "integer"},
                    "z": {"type": "integer"}, "marked": {"type": "boolean"}
                }
            })),
            tool_def("timberbot_unlock_building", "Spend science points to unlock a building prefab.", json!({
                "type": "object",
                "required": ["building"],
                "properties": {"building": {"type": "string"}}
            })),
            tool_def("timberbot_automation_link", "Wire a transmitter (sourceId) to a target building input (a..h or reset).", json!({
                "type": "object",
                "required": ["sourceId", "targetId"],
                "properties": {
                    "sourceId": {"type": "integer"},
                    "targetId": {"type": "integer"},
                    "input": {"type": "string"}
                }
            })),
            tool_def("timberbot_automation_configure", "Configure a supported automation component property.", json!({
                "type": "object",
                "required": ["id", "property", "value"],
                "properties": {
                    "id": {"type": "integer"},
                    "property": {"type": "string"},
                    "value": {"type": "string"}
                }
            })),
            // --- Official built-in Timberborn 1.0+ HTTP Lever / Adapter API (:8080) ---
            tool_def("lever_list", "List all official built-in HTTP Levers from localhost:8080/api/levers.", json!({"type": "object", "properties": {}})),
            tool_def("lever_get", "Read one official HTTP Lever state by name.", json!({
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            })),
            tool_def("lever_switch_on", "Switch an official HTTP Lever ON via /api/switch-on/{name}.", json!({
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            })),
            tool_def("lever_switch_off", "Switch an official HTTP Lever OFF via /api/switch-off/{name}.", json!({
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            })),
            tool_def("lever_set_color", "Set an official HTTP Lever indicator color (RRGGBB hex).", json!({
                "type": "object",
                "required": ["name", "color"],
                "properties": {"name": {"type": "string"}, "color": {"type": "string"}}
            })),
            tool_def("adapter_list", "List all official built-in HTTP Adapters from localhost:8080/api/adapters.", json!({"type": "object", "properties": {}})),
            tool_def("adapter_get", "Read one official HTTP Adapter state by name.", json!({
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            })),
            // --- Read-only .timber save inspection ---
            tool_def("save_inspect", "Read-only inspection of a .timber ZIP save file (world.json summary, entities, stocks, terrain).", json!({
                "type": "object",
                "required": ["path"],
                "properties": {"path": {"type": "string"}}
            })),
        ]
    }

    pub async fn handle_json_line(&self, line: &str) -> Option<String> {
        let req: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("parse error: {e}"),
                    }),
                };
                return serde_json::to_string(&resp).ok();
            }
        };

        // Notifications without an id produce no response.
        let id = req.id?;

        let response = match req.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "timberbot-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
                error: None,
            },
            "ping" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({})),
                error: None,
            },
            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({ "tools": self.tool_definitions() })),
                error: None,
            },
            "tools/call" => {
                let name = req.params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = req
                    .params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                match self.dispatch_tool(name, args).await {
                    Ok(val) => {
                        let text =
                            serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string());
                        JsonRpcResponse {
                            jsonrpc: "2.0",
                            id,
                            result: Some(json!({
                                "content": [{ "type": "text", "text": text }],
                                "structuredContent": val,
                                "isError": false
                            })),
                            error: None,
                        }
                    }
                    Err(err) => JsonRpcResponse {
                        jsonrpc: "2.0",
                        id,
                        result: Some(json!({
                            "content": [{ "type": "text", "text": err }],
                            "isError": true
                        })),
                        error: None,
                    },
                }
            }
            other => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("method not found: {other}"),
                }),
            },
        };

        serde_json::to_string(&response).ok()
    }

    pub async fn dispatch_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        match name {
            "timberbot_ping" => to_val(self.mod_client.ping().await),
            "timberbot_agent_state" => to_val(self.mod_client.agent_state().await),
            "timberbot_summary" => to_val(self.mod_client.summary().await),
            "timberbot_weather" => to_val(self.mod_client.weather().await),
            "timberbot_resources" => to_val(self.mod_client.resources().await),
            "timberbot_buildings" => {
                let q: ListQuery = serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.buildings(&q).await)
            }
            "timberbot_beavers" => {
                let q: ListQuery = serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.beavers(&q).await)
            }
            "timberbot_tiles" => {
                let q: TilesQuery = serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.tiles(&q).await)
            }
            "timberbot_alerts" => {
                let q: ListQuery = serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.alerts(&q).await)
            }
            "timberbot_science" => to_val(self.mod_client.science().await),
            "timberbot_prefabs" => to_val(self.mod_client.prefabs().await),
            "timberbot_set_speed" => {
                let speed = args
                    .get("speed")
                    .and_then(Value::as_u64)
                    .ok_or("missing speed")? as u8;
                to_val(self.mod_client.set_speed(speed).await)
            }
            "timberbot_set_workhours" => {
                let end_hours = args
                    .get("endHours")
                    .and_then(Value::as_u64)
                    .ok_or("missing endHours")? as u8;
                to_val(self.mod_client.set_workhours(end_hours).await)
            }
            "timberbot_pause_building" => {
                let req: PauseBuildingRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.pause_building(req.id, req.paused).await)
            }
            "timberbot_set_workers" => {
                let req: SetWorkersRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.set_workers(req.id, req.count).await)
            }
            "timberbot_set_priority" => {
                let req: SetPriorityRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.set_priority(&req).await)
            }
            "timberbot_set_floodgate" => {
                let req: SetFloodgateRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.set_floodgate(req.id, req.height).await)
            }
            "timberbot_set_storage" => {
                let req: SetStorageRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.set_storage(&req).await)
            }
            "timberbot_find_placement" => {
                let req: FindPlacementRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.find_placement(&req).await)
            }
            "timberbot_place_building" => {
                let req: PlaceBuildingRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.place_building(&req).await)
            }
            "timberbot_place_path" => {
                let req: PlacePathRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.place_path(&req).await)
            }
            "timberbot_demolish_building" => {
                let req: BuildingIdRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.demolish_building(req.id).await)
            }
            "timberbot_plant_crop" => {
                let req: PlantCropRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.plant_crop(&req).await)
            }
            "timberbot_mark_trees" => {
                let req: CuttingAreaRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.mark_trees(&req).await)
            }
            "timberbot_unlock_building" => {
                let req: UnlockBuildingRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.unlock_building(req.building).await)
            }
            "timberbot_automation_link" => {
                let req: AutomationLinkRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.link_automation(&req).await)
            }
            "timberbot_automation_configure" => {
                let req: AutomationConfigureRequest =
                    serde_json::from_value(args).map_err(|e| e.to_string())?;
                to_val(self.mod_client.configure_automation(&req).await)
            }
            // --- Official :8080 HTTP Levers & Adapters ---
            "lever_list" => to_val(self.lever_client.list_levers().await),
            "lever_get" => {
                let lever = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("missing name")?;
                to_val(self.lever_client.get_lever(lever).await)
            }
            "lever_switch_on" => {
                let lever = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("missing name")?;
                let body = self
                    .lever_client
                    .switch_on(lever)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({"lever": lever, "state": true, "response": body}))
            }
            "lever_switch_off" => {
                let lever = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("missing name")?;
                let body = self
                    .lever_client
                    .switch_off(lever)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({"lever": lever, "state": false, "response": body}))
            }
            "lever_set_color" => {
                let lever = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("missing name")?;
                let color = args
                    .get("color")
                    .and_then(Value::as_str)
                    .ok_or("missing color")?;
                let body = self
                    .lever_client
                    .set_color(lever, color)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(json!({"lever": lever, "color": color, "response": body}))
            }
            "adapter_list" => to_val(self.lever_client.list_adapters().await),
            "adapter_get" => {
                let adapter = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("missing name")?;
                to_val(self.lever_client.get_adapter(adapter).await)
            }
            // --- Read-only .timber save inspection ---
            "save_inspect" => {
                let raw_path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or("missing path")?;
                let path = PathBuf::from(raw_path);
                if let Some(root) = &self.allowed_save_root {
                    let canon_root = root
                        .canonicalize()
                        .map_err(|e| format!("invalid save root: {e}"))?;
                    let canon_path = path
                        .canonicalize()
                        .map_err(|e| format!("cannot resolve save path: {e}"))?;
                    if !canon_path.starts_with(&canon_root) {
                        return Err("access denied: save path is outside allowed save root".into());
                    }
                }
                let report = timberbot_save::inspect_save(&path).map_err(|e| e.to_string())?;
                serde_json::to_value(report).map_err(|e| e.to_string())
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }

    pub async fn run_stdio(&self) -> Result<()> {
        let stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();
        let mut lines = stdin.lines();
        while let Some(line) = lines.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(resp) = self.handle_json_line(trimmed).await {
                stdout.write_all(resp.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }
        Ok(())
    }
}

fn tool_def(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn to_val<T: Serialize, E: std::fmt::Display>(res: Result<T, E>) -> Result<Value, String> {
    let v = res.map_err(|e| e.to_string())?;
    serde_json::to_value(v).map_err(|e| e.to_string())
}
