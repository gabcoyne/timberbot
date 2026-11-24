use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use timberbot_protocol::http::*;
use timberbot_protocol::{DEFAULT_HTTP_PORT, DEFAULT_WS_PORT};

use crate::error::{ClientError, Result};
use crate::ws::TimberbotWsConnection;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub host: String,
    pub http_port: u16,
    pub ws_port: u16,
    pub auth_token: Option<String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            http_port: DEFAULT_HTTP_PORT,
            ws_port: DEFAULT_WS_PORT,
            auth_token: None,
        }
    }
}

impl ClientConfig {
    pub fn from_env() -> Self {
        let host = std::env::var("TBOT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let http_port = std::env::var("TBOT_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_HTTP_PORT);
        let ws_port = std::env::var("TBOT_WS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_WS_PORT);
        let auth_token = std::env::var("TBOT_AUTH_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Self {
            host,
            http_port,
            ws_port,
            auth_token,
        }
    }

    pub fn http_base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.http_port)
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}:{}/api/ws", self.host, self.ws_port)
    }
}

#[derive(Debug, Clone)]
pub struct TimberbotClient {
    config: ClientConfig,
    http: Client,
}

impl TimberbotClient {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let http = Client::builder().build()?;
        Ok(Self { config, http })
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub async fn connect_ws(&self) -> Result<TimberbotWsConnection> {
        TimberbotWsConnection::connect(&self.config.ws_url(), self.config.auth_token.as_deref())
            .await
    }

    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = &self.config.auth_token {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {trimmed}")) {
                    headers.insert(AUTHORIZATION, val);
                }
            }
        }
        headers
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&impl Serialize>,
    ) -> Result<T> {
        let url = format!("{}{}", self.config.http_base_url(), path);
        let mut req = self.http.request(method, &url).headers(self.auth_headers());
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if status == StatusCode::CONFLICT {
            if let Ok(gnr) = serde_json::from_str::<GameNotReady>(&text) {
                return Err(ClientError::GameNotReady { hint: gnr.hint });
            }
        }
        if status == StatusCode::UNAUTHORIZED {
            let message = serde_json::from_str::<ErrorResponse>(&text)
                .map(|e| e.error)
                .unwrap_or(text);
            return Err(ClientError::Unauthorized { message });
        }
        if !status.is_success() {
            let message = serde_json::from_str::<ErrorResponse>(&text)
                .map(|e| e.error)
                .unwrap_or(text);
            return Err(ClientError::ApiError {
                status: status.as_u16(),
                message,
            });
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn get_no_format<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send_json::<T>(Method::GET, path, &[], None::<&()>)
            .await
    }

    async fn get_json_format<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let q = [("format", "json".to_string())];
        self.send_json::<T>(Method::GET, path, &q, None::<&()>)
            .await
    }

    async fn get_list<T: DeserializeOwned>(&self, path: &str, query: &ListQuery) -> Result<T> {
        let mut q = vec![("format", "json".to_string())];
        if let Some(limit) = query.limit {
            q.push(("limit", limit.to_string()));
        }
        if let Some(offset) = query.offset {
            q.push(("offset", offset.to_string()));
        }
        if let Some(detail) = &query.detail {
            q.push(("detail", detail.clone()));
        }
        if let Some(id) = query.id {
            q.push(("id", id.to_string()));
        }
        if let Some(name) = &query.name {
            q.push(("name", name.clone()));
        }
        if let Some(x) = query.x {
            q.push(("x", x.to_string()));
        }
        if let Some(y) = query.y {
            q.push(("y", y.to_string()));
        }
        if let Some(radius) = query.radius {
            q.push(("radius", radius.to_string()));
        }
        self.send_json::<T>(Method::GET, path, &q, None::<&()>)
            .await
    }

    async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.send_json::<T>(Method::POST, path, &[], Some(body))
            .await
    }

    // --- Meta & gate-exempt agent routes ---

    pub async fn ping(&self) -> Result<PingResponse> {
        self.get_no_format("/api/ping").await
    }

    pub async fn settlement(&self) -> Result<SettlementName> {
        self.get_no_format("/api/settlement").await
    }

    pub async fn agent_state(&self) -> Result<AgentState> {
        self.get_no_format("/api/agent/state").await
    }

    pub async fn set_agent_config(&self, req: &AgentConfigRequest) -> Result<AgentState> {
        self.post_json("/api/agent/config", req).await
    }

    pub async fn submit_agent_request(&self, prompt: impl Into<String>) -> Result<AgentRequestAck> {
        self.post_json(
            "/api/agent/request",
            &AgentRequestBody {
                prompt: prompt.into(),
            },
        )
        .await
    }

    pub async fn set_ready(&self, ready: bool) -> Result<ReadyAck> {
        self.post_json("/api/ready", &ReadyRequest { ready }).await
    }

    pub async fn post_agent_message(&self, message: impl Into<String>) -> Result<OkAck> {
        self.post_json(
            "/api/agent/message",
            &AgentMessageRequest {
                message: message.into(),
            },
        )
        .await
    }

    // --- Read state ---

    pub async fn summary(&self) -> Result<Summary> {
        self.get_json_format("/api/summary").await
    }

    pub async fn time(&self) -> Result<TimeInfo> {
        self.get_no_format("/api/time").await
    }

    pub async fn weather(&self) -> Result<WeatherInfo> {
        self.get_no_format("/api/weather").await
    }

    pub async fn population(&self) -> Result<Vec<PopulationItem>> {
        self.get_no_format("/api/population").await
    }

    pub async fn resources(&self) -> Result<ResourcesMap> {
        self.get_json_format("/api/resources").await
    }

    pub async fn districts(&self) -> Result<Vec<District>> {
        self.get_json_format("/api/districts").await
    }

    pub async fn science(&self) -> Result<ScienceInfo> {
        self.get_json_format("/api/science").await
    }

    pub async fn wellbeing(&self) -> Result<WellbeingReport> {
        self.get_json_format("/api/wellbeing").await
    }

    pub async fn workhours(&self) -> Result<WorkHours> {
        self.get_no_format("/api/workhours").await
    }

    pub async fn speed(&self) -> Result<SpeedInfo> {
        self.get_no_format("/api/speed").await
    }

    pub async fn prefabs(&self) -> Result<Vec<PrefabInfo>> {
        self.get_no_format("/api/prefabs").await
    }

    pub async fn power(&self) -> Result<Vec<PowerNetwork>> {
        self.get_json_format("/api/power").await
    }

    pub async fn tiles(&self, query: &TilesQuery) -> Result<TilesResponse> {
        let mut q = Vec::new();
        if let Some(x1) = query.x1 {
            q.push(("x1", x1.to_string()));
        }
        if let Some(y1) = query.y1 {
            q.push(("y1", y1.to_string()));
        }
        if let Some(x2) = query.x2 {
            q.push(("x2", x2.to_string()));
        }
        if let Some(y2) = query.y2 {
            q.push(("y2", y2.to_string()));
        }
        self.send_json(Method::GET, "/api/tiles", &q, None::<&()>)
            .await
    }

    pub async fn tree_clusters(&self) -> Result<Vec<Cluster>> {
        self.get_json_format("/api/tree_clusters").await
    }

    pub async fn food_clusters(&self) -> Result<Vec<Cluster>> {
        self.get_json_format("/api/food_clusters").await
    }

    pub async fn alerts(&self, query: &ListQuery) -> Result<Vec<AlertItem>> {
        self.get_list("/api/alerts", query).await
    }

    pub async fn notifications(&self, query: &ListQuery) -> Result<Vec<NotificationItem>> {
        self.get_list("/api/notifications", query).await
    }

    pub async fn distribution(&self) -> Result<Vec<DistrictDistribution>> {
        self.get_json_format("/api/distribution").await
    }

    pub async fn buildings(&self, query: &ListQuery) -> Result<PaginatedList<Building>> {
        self.get_list("/api/buildings", query).await
    }

    pub async fn beavers(&self, query: &ListQuery) -> Result<PaginatedList<Beaver>> {
        self.get_list("/api/beavers", query).await
    }

    pub async fn trees(&self, query: &ListQuery) -> Result<PaginatedList<Plant>> {
        self.get_list("/api/trees", query).await
    }

    pub async fn crops(&self, query: &ListQuery) -> Result<PaginatedList<Plant>> {
        self.get_list("/api/crops", query).await
    }

    pub async fn gatherables(&self, query: &ListQuery) -> Result<PaginatedList<Gatherable>> {
        self.get_list("/api/gatherables", query).await
    }

    // --- Write actions ---

    pub async fn set_speed(&self, speed: u8) -> Result<Value> {
        self.post_json("/api/speed", &SpeedInfo { speed }).await
    }

    pub async fn set_workhours(&self, end_hours: u8) -> Result<Value> {
        self.post_json("/api/workhours", &SetWorkHoursRequest { end_hours })
            .await
    }

    pub async fn pause_building(&self, id: i64, paused: bool) -> Result<Value> {
        self.post_json("/api/building/pause", &PauseBuildingRequest { id, paused })
            .await
    }

    pub async fn set_clutch(&self, id: i64, engaged: bool) -> Result<Value> {
        self.post_json("/api/building/clutch", &SetClutchRequest { id, engaged })
            .await
    }

    pub async fn set_floodgate(&self, id: i64, height: f64) -> Result<Value> {
        self.post_json(
            "/api/building/floodgate",
            &SetFloodgateRequest { id, height },
        )
        .await
    }

    pub async fn set_priority(&self, req: &SetPriorityRequest) -> Result<Value> {
        self.post_json("/api/building/priority", req).await
    }

    pub async fn set_haul_priority(&self, id: i64, prioritized: bool) -> Result<Value> {
        self.post_json(
            "/api/building/hauling",
            &SetHaulingRequest { id, prioritized },
        )
        .await
    }

    pub async fn set_recipe(&self, id: i64, recipe: impl Into<String>) -> Result<Value> {
        self.post_json(
            "/api/building/recipe",
            &SetRecipeRequest {
                id,
                recipe: recipe.into(),
            },
        )
        .await
    }

    pub async fn set_farmhouse_action(&self, id: i64, action: impl Into<String>) -> Result<Value> {
        self.post_json(
            "/api/building/farmhouse",
            &SetFarmhouseRequest {
                id,
                action: action.into(),
            },
        )
        .await
    }

    pub async fn set_plantable_priority(
        &self,
        id: i64,
        plantable: impl Into<String>,
    ) -> Result<Value> {
        self.post_json(
            "/api/building/plantable",
            &SetPlantableRequest {
                id,
                plantable: plantable.into(),
            },
        )
        .await
    }

    pub async fn set_workers(&self, id: i64, count: u32) -> Result<Value> {
        self.post_json("/api/building/workers", &SetWorkersRequest { id, count })
            .await
    }

    pub async fn building_range(&self, id: i64) -> Result<Value> {
        self.post_json("/api/building/range", &BuildingIdRequest { id })
            .await
    }

    pub async fn set_storage(&self, req: &SetStorageRequest) -> Result<Value> {
        self.post_json("/api/building/storage", req).await
    }

    pub async fn demolish_building(&self, id: i64) -> Result<Value> {
        self.post_json("/api/building/demolish", &BuildingIdRequest { id })
            .await
    }

    pub async fn demolish_crop(&self, id: i64) -> Result<Value> {
        self.post_json("/api/crop/demolish", &BuildingIdRequest { id })
            .await
    }

    pub async fn place_building(&self, req: &PlaceBuildingRequest) -> Result<Value> {
        self.post_json("/api/building/place", req).await
    }

    pub async fn find_placement(
        &self,
        req: &FindPlacementRequest,
    ) -> Result<FindPlacementResponse> {
        self.post_json("/api/placement/find", req).await
    }

    pub async fn place_path(&self, req: &PlacePathRequest) -> Result<Value> {
        self.post_json("/api/path/place", req).await
    }

    pub async fn plant_crop(&self, req: &PlantCropRequest) -> Result<Value> {
        self.post_json("/api/planting/mark", req).await
    }

    pub async fn find_planting(&self, req: &FindPlantingRequest) -> Result<Value> {
        self.post_json("/api/planting/find", req).await
    }

    pub async fn clear_planting(&self, req: &ClearPlantingRequest) -> Result<Value> {
        self.post_json("/api/planting/clear", req).await
    }

    pub async fn mark_trees(&self, req: &CuttingAreaRequest) -> Result<Value> {
        self.post_json("/api/cutting/area", req).await
    }

    pub async fn migrate(&self, req: &MigrateRequest) -> Result<Value> {
        self.post_json("/api/district/migrate", req).await
    }

    pub async fn unlock_building(&self, building: impl Into<String>) -> Result<Value> {
        self.post_json(
            "/api/science/unlock",
            &UnlockBuildingRequest {
                building: building.into(),
            },
        )
        .await
    }

    pub async fn set_distribution(&self, req: &SetDistributionRequest) -> Result<Value> {
        self.post_json("/api/distribution", req).await
    }

    pub async fn link_automation(&self, req: &AutomationLinkRequest) -> Result<Value> {
        self.post_json("/api/automation/link", req).await
    }

    pub async fn unlink_automation(&self, req: &AutomationUnlinkRequest) -> Result<Value> {
        self.post_json("/api/automation/unlink", req).await
    }

    pub async fn configure_automation(&self, req: &AutomationConfigureRequest) -> Result<Value> {
        self.post_json("/api/automation/configure", req).await
    }

    pub async fn rename_automation(&self, req: &AutomationRenameRequest) -> Result<Value> {
        self.post_json("/api/automation/rename", req).await
    }
}
