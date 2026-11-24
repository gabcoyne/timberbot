use std::sync::Arc;

use axum::extract::{Query, Request, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use subtle::ConstantTimeEq;
use timberbot_protocol::http::*;
use timberbot_protocol::ws::ServerFrame;
use timberbot_protocol::{
    is_gate_exempt, GAME_NOT_READY_ERROR, GAME_NOT_READY_HINT, OPENAPI_VERSION,
};

use crate::SharedMockState;

pub fn build_router(state: Arc<SharedMockState>) -> Router {
    Router::new()
        // Meta & gate-exempt agent routes
        .route("/api/ping", get(ping))
        .route("/api/settlement", get(settlement))
        .route("/api/agent/state", get(agent_state))
        .route("/api/agent/config", post(agent_config))
        .route("/api/agent/request", post(agent_request))
        .route("/api/agent/message", post(agent_message))
        .route("/api/ready", post(set_ready))
        // Read endpoints
        .route("/api/summary", get(summary))
        .route("/api/time", get(time_ep))
        .route("/api/weather", get(weather))
        .route("/api/population", get(population))
        .route("/api/resources", get(resources))
        .route("/api/districts", get(districts))
        .route("/api/science", get(science))
        .route("/api/wellbeing", get(wellbeing))
        .route("/api/workhours", get(workhours).post(set_workhours))
        .route("/api/speed", get(speed).post(set_speed))
        .route("/api/prefabs", get(prefabs))
        .route("/api/power", get(power))
        .route("/api/tiles", get(tiles))
        .route("/api/tree_clusters", get(tree_clusters))
        .route("/api/food_clusters", get(food_clusters))
        .route("/api/alerts", get(alerts))
        .route("/api/notifications", get(notifications))
        .route(
            "/api/distribution",
            get(distribution).post(set_distribution),
        )
        .route("/api/buildings", get(buildings))
        .route("/api/beavers", get(beavers))
        .route("/api/trees", get(trees))
        .route("/api/crops", get(crops))
        .route("/api/gatherables", get(gatherables))
        // Write endpoints
        .route("/api/building/pause", post(pause_building))
        .route("/api/building/clutch", post(set_clutch))
        .route("/api/building/floodgate", post(set_floodgate))
        .route("/api/building/priority", post(set_priority))
        .route("/api/building/hauling", post(set_hauling))
        .route("/api/building/recipe", post(set_recipe))
        .route("/api/building/farmhouse", post(set_farmhouse))
        .route("/api/building/plantable", post(set_plantable))
        .route("/api/building/workers", post(set_workers))
        .route("/api/building/range", post(building_range))
        .route("/api/building/storage", post(set_storage))
        .route("/api/building/demolish", post(demolish_building))
        .route("/api/crop/demolish", post(demolish_crop))
        .route("/api/building/place", post(place_building))
        .route("/api/placement/find", post(find_placement))
        .route("/api/path/place", post(place_path))
        .route("/api/planting/mark", post(plant_crop))
        .route("/api/planting/find", post(find_planting))
        .route("/api/planting/clear", post(clear_planting))
        .route("/api/cutting/area", post(cutting_area))
        .route("/api/district/migrate", post(migrate))
        .route("/api/science/unlock", post(unlock_building))
        .route("/api/automation/link", post(automation_link))
        .route("/api/automation/unlink", post(automation_unlink))
        .route("/api/automation/configure", post(automation_configure))
        .route("/api/automation/rename", post(automation_rename))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            auth_and_ready_gate,
        ))
        .with_state(state)
}

pub fn token_matches(expected: &str, presented: &str) -> bool {
    let eb = expected.as_bytes();
    let pb = presented.as_bytes();
    if eb.len() != pb.len() || eb.is_empty() {
        return false;
    }
    bool::from(eb.ct_eq(pb))
}

pub fn extract_bearer(header_val: Option<&str>) -> Option<&str> {
    let raw = header_val?.trim();
    if raw.len() <= 7 {
        return None;
    }
    let (scheme, rest) = raw.split_at(7);
    if !scheme.eq_ignore_ascii_case("bearer ") {
        return None;
    }
    let token = rest.trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

async fn auth_and_ready_gate(
    State(state): State<Arc<SharedMockState>>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().trim_end_matches('/').to_ascii_lowercase();

    // 1. Bearer auth on every /api/* except /api/ping
    if !state.auth_token.is_empty() && path != "/api/ping" && path.starts_with("/api/") {
        let auth_hdr = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        let presented = extract_bearer(auth_hdr);
        if !presented.is_some_and(|tok| token_matches(&state.auth_token, tok)) {
            let mut resp = (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized: missing or invalid bearer token"})),
            )
                .into_response();
            resp.headers_mut().insert(
                WWW_AUTHENTICATE,
                HeaderValue::from_static("Bearer realm=\"timberbot\""),
            );
            return resp;
        }
    }

    // 2. Ready gate on every non-exempt /api/* route
    if path.starts_with("/api/") && !is_gate_exempt(&path) {
        let is_ready = state.world.lock().expect("lock").agent_state.ready;
        if !is_ready {
            return (
                StatusCode::CONFLICT,
                Json(GameNotReady {
                    error: GAME_NOT_READY_ERROR.to_string(),
                    hint: GAME_NOT_READY_HINT.to_string(),
                }),
            )
                .into_response();
        }
    }

    next.run(req).await
}

// --- Handlers ---

async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        status: "ok".to_string(),
        ready: true,
        openapi_version: OPENAPI_VERSION.to_string(),
    })
}

async fn settlement(State(s): State<Arc<SharedMockState>>) -> Json<SettlementName> {
    let w = s.world.lock().expect("lock");
    Json(SettlementName {
        name: w.settlement.clone(),
    })
}

async fn agent_state(State(s): State<Arc<SharedMockState>>) -> Json<AgentState> {
    let w = s.world.lock().expect("lock");
    Json(w.agent_state.clone())
}

async fn agent_config(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<AgentConfigRequest>,
) -> Json<AgentState> {
    let new_state = {
        let mut w = s.world.lock().expect("lock");
        if let Some(m) = req.mode {
            w.agent_state.mode = m;
        }
        if let Some(g) = req.goal {
            w.agent_state.goal = g;
        }
        w.agent_state.clone()
    };
    s.broadcast_events(vec![ServerFrame::State(new_state.clone())]);
    Json(new_state)
}

async fn agent_request(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<AgentRequestBody>,
) -> Response {
    if req.prompt.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_prompt: prompt is required"})),
        )
            .into_response();
    }
    let (pending, state_snap) = {
        let mut w = s.world.lock().expect("lock");
        let p = w.enqueue_request(req.prompt);
        (p, w.agent_state.clone())
    };
    s.broadcast_events(vec![ServerFrame::State(state_snap)]);
    Json(AgentRequestAck {
        pending_request: pending,
    })
    .into_response()
}

async fn agent_message(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<AgentMessageRequest>,
) -> Response {
    if req.message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_message: 'message' is required"})),
        )
            .into_response();
    }
    let mut w = s.world.lock().expect("lock");
    w.agent_messages.push(req.message);
    Json(OkAck { ok: true }).into_response()
}

async fn set_ready(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<ReadyRequest>,
) -> Json<ReadyAck> {
    let state_snap = {
        let mut w = s.world.lock().expect("lock");
        w.agent_state.ready = req.ready;
        w.agent_state.clone()
    };
    s.broadcast_events(vec![ServerFrame::State(state_snap)]);
    Json(ReadyAck { ready: req.ready })
}

async fn summary(State(s): State<Arc<SharedMockState>>) -> Json<Summary> {
    let w = s.world.lock().expect("lock");
    Json(w.summary())
}

async fn time_ep(State(s): State<Arc<SharedMockState>>) -> Json<TimeInfo> {
    let w = s.world.lock().expect("lock");
    Json(w.time_info())
}

async fn weather(State(s): State<Arc<SharedMockState>>) -> Json<WeatherInfo> {
    let w = s.world.lock().expect("lock");
    Json(w.weather_info())
}

async fn population(State(s): State<Arc<SharedMockState>>) -> Json<Vec<PopulationItem>> {
    let w = s.world.lock().expect("lock");
    let c = w.population_counts();
    Json(vec![PopulationItem {
        district: w.district_name.clone(),
        adults: c.adults,
        children: c.children,
        bots: c.bots,
    }])
}

async fn resources(State(s): State<Arc<SharedMockState>>) -> Json<ResourcesMap> {
    let w = s.world.lock().expect("lock");
    Json(w.resources_map())
}

async fn districts(State(s): State<Arc<SharedMockState>>) -> Json<Vec<District>> {
    let w = s.world.lock().expect("lock");
    let res = w
        .resources_map()
        .remove(&w.district_name)
        .unwrap_or_default();
    Json(vec![District {
        name: w.district_name.clone(),
        population: w.population_counts(),
        resources: res,
    }])
}

async fn science(State(s): State<Arc<SharedMockState>>) -> Json<ScienceInfo> {
    let w = s.world.lock().expect("lock");
    Json(ScienceInfo {
        points: w.science,
        unlockables: w.unlockables.clone(),
    })
}

async fn wellbeing(State(s): State<Arc<SharedMockState>>) -> Json<WellbeingReport> {
    let w = s.world.lock().expect("lock");
    let avg = if w.beavers.is_empty() {
        0.0
    } else {
        w.beavers
            .iter()
            .map(|b| b.wellbeing.unwrap_or(0) as f64)
            .sum::<f64>()
            / w.beavers.len() as f64
    };
    Json(WellbeingReport {
        beavers: w.beavers.len() as u32,
        categories: vec![WellbeingCategory {
            group: "BasicNeeds".to_string(),
            current: avg,
            max: 16.0,
            needs: vec![
                WellbeingNeed {
                    id: "Thirst".to_string(),
                    favorable_wellbeing: 1.0,
                    unfavorable_wellbeing: -4.0,
                },
                WellbeingNeed {
                    id: "Hunger".to_string(),
                    favorable_wellbeing: 1.0,
                    unfavorable_wellbeing: -4.0,
                },
            ],
        }],
    })
}

async fn workhours(State(s): State<Arc<SharedMockState>>) -> Json<WorkHours> {
    let w = s.world.lock().expect("lock");
    Json(WorkHours {
        end_hours: w.work_end_hours as f64,
        are_working_hours: w.day_progress < (w.work_end_hours as f64 / 24.0),
    })
}

async fn set_workhours(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetWorkHoursRequest>,
) -> Response {
    if !(1..=24).contains(&req.end_hours) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_param: endHours must be 1..=24"})),
        )
            .into_response();
    }
    let mut w = s.world.lock().expect("lock");
    w.work_end_hours = req.end_hours;
    Json(json!({"endHours": req.end_hours})).into_response()
}

async fn speed(State(s): State<Arc<SharedMockState>>) -> Json<SpeedInfo> {
    let w = s.world.lock().expect("lock");
    Json(SpeedInfo { speed: w.speed })
}

async fn set_speed(State(s): State<Arc<SharedMockState>>, Json(req): Json<SpeedInfo>) -> Response {
    if req.speed > 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_param: speed must be 0, 1, 2, or 3"})),
        )
            .into_response();
    }
    let day = {
        let mut w = s.world.lock().expect("lock");
        w.speed = req.speed;
        w.day_number
    };
    s.broadcast_events(vec![ServerFrame::Event(
        timberbot_protocol::ws::GameEventPayload {
            event: "speed.changed".to_string(),
            day,
            timestamp: 0,
            data: Some(json!({"speed": req.speed})),
        },
    )]);
    Json(json!({"speed": req.speed})).into_response()
}

async fn prefabs(State(s): State<Arc<SharedMockState>>) -> Json<Vec<PrefabInfo>> {
    let w = s.world.lock().expect("lock");
    Json(w.prefabs.clone())
}

async fn power() -> Json<Vec<PowerNetwork>> {
    Json(Vec::new())
}

async fn tiles(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<TilesQuery>,
) -> Json<TilesResponse> {
    let w = s.world.lock().expect("lock");
    Json(w.tiles(&q))
}

async fn tree_clusters(State(s): State<Arc<SharedMockState>>) -> Json<Vec<Cluster>> {
    let w = s.world.lock().expect("lock");
    Json(w.tree_clusters())
}

async fn food_clusters(State(s): State<Arc<SharedMockState>>) -> Json<Vec<Cluster>> {
    let w = s.world.lock().expect("lock");
    Json(w.food_clusters())
}

async fn alerts(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<ListQuery>,
) -> Json<Vec<AlertItem>> {
    let w = s.world.lock().expect("lock");
    let items = paginate_slice(&w.alerts(), &q);
    Json(items)
}

async fn notifications(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<ListQuery>,
) -> Json<Vec<NotificationItem>> {
    let w = s.world.lock().expect("lock");
    let items = paginate_slice(&w.notifications, &q);
    Json(items)
}

async fn distribution(State(s): State<Arc<SharedMockState>>) -> Json<Vec<DistrictDistribution>> {
    let w = s.world.lock().expect("lock");
    Json(w.distribution.clone())
}

async fn set_distribution(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetDistributionRequest>,
) -> Json<Value> {
    let mut w = s.world.lock().expect("lock");
    if let Some(d) = w
        .distribution
        .iter_mut()
        .find(|d| d.district == req.district)
    {
        if let Some(g) = d.goods.iter_mut().find(|g| g.good == req.good) {
            if let Some(imp) = req.import_option.filter(|v| !v.is_empty()) {
                g.import_option = imp;
            }
            if req.export_threshold >= 0 {
                g.export_threshold = req.export_threshold as f64;
            }
        } else {
            d.goods.push(GoodDistribution {
                good: req.good.clone(),
                import_option: req.import_option.unwrap_or_else(|| "Auto".to_string()),
                export_threshold: req.export_threshold.max(0) as f64,
            });
        }
    }
    Json(json!({"district": req.district, "good": req.good, "ok": true}))
}

async fn buildings(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<ListQuery>,
) -> Json<PaginatedList<Building>> {
    let w = s.world.lock().expect("lock");
    let filtered: Vec<Building> = w
        .buildings
        .iter()
        .filter(|b| q.id.is_none_or(|id| b.id == id))
        .filter(|b| {
            q.name.as_ref().is_none_or(|n| {
                b.name
                    .to_ascii_lowercase()
                    .contains(&n.to_ascii_lowercase())
            })
        })
        .filter(|b| in_radius(b.x, b.y, &q))
        .cloned()
        .collect();
    Json(paginate_vec(filtered, &q))
}

async fn beavers(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<ListQuery>,
) -> Json<PaginatedList<Beaver>> {
    let w = s.world.lock().expect("lock");
    let filtered: Vec<Beaver> = w
        .beavers
        .iter()
        .filter(|b| q.id.is_none_or(|id| b.id == id))
        .filter(|b| {
            q.name.as_ref().is_none_or(|n| {
                b.name
                    .to_ascii_lowercase()
                    .contains(&n.to_ascii_lowercase())
            })
        })
        .filter(|b| in_radius(b.x, b.y, &q))
        .cloned()
        .collect();
    Json(paginate_vec(filtered, &q))
}

async fn trees(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<ListQuery>,
) -> Json<PaginatedList<Plant>> {
    let w = s.world.lock().expect("lock");
    let filtered: Vec<Plant> = w
        .trees
        .iter()
        .filter(|t| {
            q.name.as_ref().is_none_or(|n| {
                t.name
                    .to_ascii_lowercase()
                    .contains(&n.to_ascii_lowercase())
            })
        })
        .filter(|t| in_radius(t.x, t.y, &q))
        .cloned()
        .collect();
    Json(paginate_vec(filtered, &q))
}

async fn crops(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<ListQuery>,
) -> Json<PaginatedList<Plant>> {
    let w = s.world.lock().expect("lock");
    let filtered: Vec<Plant> = w
        .crops
        .iter()
        .filter(|c| {
            q.name.as_ref().is_none_or(|n| {
                c.name
                    .to_ascii_lowercase()
                    .contains(&n.to_ascii_lowercase())
            })
        })
        .filter(|c| in_radius(c.x, c.y, &q))
        .cloned()
        .collect();
    Json(paginate_vec(filtered, &q))
}

async fn gatherables(
    State(s): State<Arc<SharedMockState>>,
    Query(q): Query<ListQuery>,
) -> Json<PaginatedList<Gatherable>> {
    let w = s.world.lock().expect("lock");
    let filtered: Vec<Gatherable> = w
        .gatherables
        .iter()
        .filter(|g| {
            q.name.as_ref().is_none_or(|n| {
                g.name
                    .to_ascii_lowercase()
                    .contains(&n.to_ascii_lowercase())
            })
        })
        .filter(|g| in_radius(g.x, g.y, &q))
        .cloned()
        .collect();
    Json(paginate_vec(filtered, &q))
}

async fn pause_building(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<PauseBuildingRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    b.paused = if req.paused { 1 } else { 0 };
    let name = b.name.clone();
    w.rebalance_workers_and_housing();
    Json(json!({"id": req.id, "name": name, "paused": req.paused})).into_response()
}

async fn set_clutch(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetClutchRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    if b.is_clutch != Some(1) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_type: not a clutch", "id": req.id, "name": b.name})),
        )
            .into_response();
    }
    b.clutch_engaged = Some(if req.engaged { 1 } else { 0 });
    Json(json!({"id": req.id, "name": b.name, "engaged": req.engaged})).into_response()
}

async fn set_floodgate(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetFloodgateRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    if b.floodgate != Some(1) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_type: not a floodgate. use buildings name:Floodgate to find floodgates",
                "id": req.id,
                "name": b.name
            })),
        )
            .into_response();
    }
    let max_h = b.max_height.unwrap_or(1.0);
    let clamped = req.height.clamp(0.0, max_h);
    b.height = Some(clamped);
    Json(json!({"id": req.id, "name": b.name, "height": clamped, "maxHeight": max_h}))
        .into_response()
}

async fn set_priority(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetPriorityRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    match req.priority_type.as_deref().unwrap_or("") {
        "construction" => b.construction_priority = Some(req.priority),
        "workplace" => b.workplace_priority = Some(req.priority),
        _ => {
            if b.finished == 0 {
                b.construction_priority = Some(req.priority);
            } else {
                b.workplace_priority = Some(req.priority);
            }
        }
    }
    let name = b.name.clone();
    w.rebalance_workers_and_housing();
    Json(json!({"id": req.id, "name": name, "priority": req.priority})).into_response()
}

async fn set_hauling(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetHaulingRequest>,
) -> Response {
    let w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    Json(json!({"id": req.id, "name": b.name, "prioritized": req.prioritized})).into_response()
}

async fn set_recipe(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetRecipeRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    b.current_recipe = if req.recipe == "none" {
        Some(String::new())
    } else {
        Some(req.recipe.clone())
    };
    Json(json!({"id": req.id, "name": b.name, "recipe": req.recipe})).into_response()
}

async fn set_farmhouse(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetFarmhouseRequest>,
) -> Response {
    if req.action != "planting" && req.action != "harvesting" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_param: action must be planting or harvesting"})),
        )
            .into_response();
    }
    let w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    Json(json!({"id": req.id, "name": b.name, "action": req.action})).into_response()
}

async fn set_plantable(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetPlantableRequest>,
) -> Response {
    let w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    Json(json!({"id": req.id, "name": b.name, "plantable": req.plantable})).into_response()
}

async fn set_workers(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetWorkersRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    let max_w = b.max_workers.unwrap_or(0);
    let clamped = req.count.min(max_w);
    b.desired_workers = Some(clamped);
    let name = b.name.clone();
    w.rebalance_workers_and_housing();
    Json(json!({"id": req.id, "name": name, "desiredWorkers": clamped, "maxWorkers": max_w}))
        .into_response()
}

async fn building_range(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<BuildingIdRequest>,
) -> Response {
    let w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    Json(json!({
        "id": b.id,
        "name": b.name,
        "center": {"x": b.x, "y": b.y, "z": b.z},
        "radius": 10
    }))
    .into_response()
}

async fn set_storage(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<SetStorageRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    if let Some(g) = req.good {
        b.allowed_good = Some(if g == "none" { String::new() } else { g });
    }
    if let Some(m) = req.mode {
        b.storage_mode = Some(m);
    }
    Json(json!({
        "id": b.id,
        "name": b.name,
        "allowedGood": b.allowed_good,
        "storageMode": b.storage_mode
    }))
    .into_response()
}

async fn demolish_building(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<BuildingIdRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(idx) = w.buildings.iter().position(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    let removed = w.buildings.remove(idx);
    w.rebalance_workers_and_housing();
    Json(json!({"id": removed.id, "name": removed.name, "demolished": true})).into_response()
}

async fn demolish_crop(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<BuildingIdRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(idx) = w.crops.iter().position(|c| c.id == req.id) else {
        return not_found(req.id);
    };
    let removed = w.crops.remove(idx);
    Json(json!({"id": removed.id, "name": removed.name, "demolished": true})).into_response()
}

async fn place_building(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<PlaceBuildingRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    match w.place_building(&req) {
        Ok(val) => Json(val).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(json!({"error": err}))).into_response(),
    }
}

async fn find_placement(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<FindPlacementRequest>,
) -> Json<FindPlacementResponse> {
    let w = s.world.lock().expect("lock");
    Json(w.find_placement(&req))
}

async fn place_path(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<PlacePathRequest>,
) -> Json<Value> {
    let mut w = s.world.lock().expect("lock");
    let mut placed = 0usize;
    let mut x = req.x1;
    let mut y = req.y1;
    while x != req.x2 || y != req.y2 {
        if x < req.x2 {
            x += 1;
        } else if x > req.x2 {
            x -= 1;
        } else if y < req.y2 {
            y += 1;
        } else if y > req.y2 {
            y -= 1;
        }
        let pr = PlaceBuildingRequest {
            prefab: "Path.Folktails".to_string(),
            x,
            y,
            z: 2,
            orientation: Orientation::South,
        };
        if w.place_building(&pr).is_ok() {
            placed += 1;
        }
    }
    Json(json!({"placed": placed, "style": req.style}))
}

async fn plant_crop(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<PlantCropRequest>,
) -> Json<Value> {
    let mut w = s.world.lock().expect("lock");
    let mut count = 0usize;
    for y in req.y1.min(req.y2)..=req.y1.max(req.y2) {
        for x in req.x1.min(req.x2)..=req.x1.max(req.x2) {
            if !w.crops.iter().any(|c| c.x == x && c.y == y) {
                let id = w.alloc_id();
                w.crops.push(Plant {
                    id,
                    name: req.crop.clone(),
                    x,
                    y,
                    z: req.z,
                    alive: 1,
                    grown: Some(0),
                    growth: Some(0.0),
                    marked: Some(0),
                });
                count += 1;
            }
        }
    }
    Json(json!({"crop": req.crop, "planted": count}))
}

async fn find_planting(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<FindPlantingRequest>,
) -> Json<Value> {
    let w = s.world.lock().expect("lock");
    let mut spots = Vec::new();
    for y in 14..22 {
        for x in 13..15 {
            if !w.crops.iter().any(|c| c.x == x && c.y == y)
                && !w.buildings.iter().any(|b| b.x == x && b.y == y)
            {
                spots.push(json!({"x": x, "y": y, "z": 2}));
            }
        }
    }
    Json(json!({"crop": req.crop, "spots": spots}))
}

async fn clear_planting(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<ClearPlantingRequest>,
) -> Json<Value> {
    let mut w = s.world.lock().expect("lock");
    let before = w.crops.len();
    let (x1, x2) = (req.x1.min(req.x2), req.x1.max(req.x2));
    let (y1, y2) = (req.y1.min(req.y2), req.y1.max(req.y2));
    w.crops
        .retain(|c| !(c.x >= x1 && c.x <= x2 && c.y >= y1 && c.y <= y2));
    Json(json!({"cleared": before - w.crops.len()}))
}

async fn cutting_area(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<CuttingAreaRequest>,
) -> Json<Value> {
    let mut w = s.world.lock().expect("lock");
    let (x1, x2) = (req.x1.min(req.x2), req.x1.max(req.x2));
    let (y1, y2) = (req.y1.min(req.y2), req.y1.max(req.y2));
    let mut affected = 0usize;
    for t in &mut w.trees {
        if t.x >= x1 && t.x <= x2 && t.y >= y1 && t.y <= y2 {
            t.marked = Some(if req.marked { 1 } else { 0 });
            affected += 1;
        }
    }
    Json(json!({"marked": req.marked, "affected": affected}))
}

async fn migrate(Json(req): Json<MigrateRequest>) -> Json<Value> {
    Json(json!({"from": req.from, "to": req.to, "migrated": req.count}))
}

async fn unlock_building(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<UnlockBuildingRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(idx) = w.unlockables.iter().position(|u| u.name == req.building) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("not_found: unknown unlockable '{}'", req.building)})),
        )
            .into_response();
    };
    if w.unlockables[idx].unlocked == 1 {
        return Json(json!({"building": req.building, "alreadyUnlocked": true})).into_response();
    }
    let cost = w.unlockables[idx].cost;
    if w.science < cost {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("insufficient_science: need {cost}, have {}", w.science)
            })),
        )
            .into_response();
    }
    w.science -= cost;
    w.unlockables[idx].unlocked = 1;
    if let Some(p) = w.prefabs.iter_mut().find(|p| p.name == req.building) {
        p.unlocked = true;
    }
    Json(json!({"building": req.building, "unlocked": true, "scienceRemaining": w.science}))
        .into_response()
}

async fn automation_link(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<AutomationLinkRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(src_name) = w
        .buildings
        .iter()
        .find(|b| b.id == req.source_id)
        .map(|b| b.name.clone())
    else {
        return not_found(req.source_id);
    };
    let Some(tgt) = w.buildings.iter_mut().find(|b| b.id == req.target_id) else {
        return not_found(req.target_id);
    };
    if let Some(auto) = &mut tgt.automation {
        auto.is_automated = Some(1);
        auto.input_name = Some(src_name.clone());
    }
    Json(json!({
        "sourceId": req.source_id,
        "targetId": req.target_id,
        "input": req.input,
        "linked": true
    }))
    .into_response()
}

async fn automation_unlink(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<AutomationUnlinkRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(tgt) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    if let Some(auto) = &mut tgt.automation {
        auto.is_automated = Some(0);
        auto.input_name = Some(String::new());
    }
    Json(json!({"id": req.id, "input": req.input, "unlinked": true})).into_response()
}

async fn automation_configure(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<AutomationConfigureRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    let auto_type = b
        .automation
        .as_ref()
        .and_then(|a| a.automation_type.clone())
        .unwrap_or_default();
    if auto_type == "Automatable" || auto_type.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid_type: building has no automation component", "id": req.id})),
        )
            .into_response();
    }
    if let Some(auto) = &mut b.automation {
        auto.config
            .insert(req.property.clone(), Value::String(req.value.clone()));
    }
    Json(json!({
        "id": b.id,
        "name": b.name,
        "property": req.property,
        "value": req.value,
        "automationType": auto_type
    }))
    .into_response()
}

async fn automation_rename(
    State(s): State<Arc<SharedMockState>>,
    Json(req): Json<AutomationRenameRequest>,
) -> Response {
    let mut w = s.world.lock().expect("lock");
    let Some(b) = w.buildings.iter_mut().find(|b| b.id == req.id) else {
        return not_found(req.id);
    };
    if let Some(auto) = &mut b.automation {
        auto.automator_name = Some(req.name.clone());
    }
    Json(json!({"id": b.id, "name": req.name})).into_response()
}

fn not_found(id: i64) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": format!("not_found: no entity with id {id}"), "id": id})),
    )
        .into_response()
}

fn in_radius(x: i32, y: i32, q: &ListQuery) -> bool {
    if let (Some(cx), Some(cy)) = (q.x, q.y) {
        let r = q.radius.unwrap_or(30);
        (x - cx).abs() + (y - cy).abs() <= r
    } else {
        true
    }
}

fn paginate_vec<T>(items: Vec<T>, q: &ListQuery) -> PaginatedList<T> {
    let total = items.len();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(100);
    let iter = items.into_iter().skip(offset);
    let page: Vec<T> = if limit == 0 {
        iter.collect()
    } else {
        iter.take(limit).collect()
    };
    PaginatedList {
        total,
        offset,
        limit,
        items: page,
    }
}

fn paginate_slice<T: Clone>(items: &[T], q: &ListQuery) -> Vec<T> {
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(100);
    let iter = items.iter().skip(offset).cloned();
    if limit == 0 {
        iter.collect()
    } else {
        iter.take(limit).collect()
    }
}
