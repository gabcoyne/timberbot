use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {
    pub status: String,
    pub ready: bool,
    pub openapi_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettlementName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameNotReady {
    pub error: String,
    pub hint: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    Autonomous,
    Request,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingRequest {
    pub id: i64,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentState {
    pub mode: AgentMode,
    pub goal: String,
    pub ready: bool,
    pub pending_request: Option<PendingRequest>,
    pub agent_status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<AgentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentRequestBody {
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRequestAck {
    pub pending_request: PendingRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadyRequest {
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadyAck {
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentMessageRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OkAck {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopulationCounts {
    pub adults: u32,
    pub children: u32,
    pub bots: u32,
}

impl PopulationCounts {
    pub fn total_beavers(&self) -> u32 {
        self.adults + self.children
    }

    pub fn total(&self) -> u32 {
        self.adults + self.children + self.bots
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopulationItem {
    pub district: String,
    pub adults: u32,
    pub children: u32,
    pub bots: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoodStock {
    pub available: i64,
    pub all: i64,
}

pub type DistrictResources = BTreeMap<String, GoodStock>;
pub type ResourcesMap = BTreeMap<String, DistrictResources>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeInfo {
    pub day_number: u32,
    pub day_progress: f64,
    pub partial_day_number: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SummaryTime {
    pub day_number: u32,
    pub day_progress: f64,
    pub partial_day_number: f64,
    #[serde(default)]
    pub speed: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeatherInfo {
    pub cycle: u32,
    pub cycle_day: u32,
    pub cycle_length_in_days: u32,
    pub is_hazardous: bool,
    pub temperate_weather_duration: u32,
    pub hazardous_weather_duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkHours {
    pub end_hours: f64,
    pub are_working_hours: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetWorkHoursRequest {
    pub end_hours: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeedInfo {
    pub speed: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DistrictHousing {
    pub occupied_beds: u32,
    pub total_beds: u32,
    pub homeless: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistrictEmployment {
    pub assigned: u32,
    pub vacancies: u32,
    pub unemployed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistrictWellbeing {
    pub average: f64,
    pub critical: u32,
    pub miserable: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DistrictCenterLocation {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub entrance_x: i32,
    pub entrance_y: i32,
    pub orientation: Orientation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryDistrict {
    pub name: String,
    pub population: PopulationCounts,
    #[serde(default)]
    pub resources: BTreeMap<String, i64>,
    #[serde(default)]
    pub housing: Option<DistrictHousing>,
    #[serde(default)]
    pub employment: Option<DistrictEmployment>,
    #[serde(default)]
    pub wellbeing: Option<DistrictWellbeing>,
    #[serde(default)]
    pub dc: Option<DistrictCenterLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryCrops {
    pub growing: u32,
    pub ready: u32,
    #[serde(default)]
    pub species: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cluster {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub total: u32,
    pub grown: u32,
    #[serde(default)]
    pub species: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub settlement: String,
    pub faction: String,
    pub science: u32,
    #[serde(default)]
    pub alerts: BTreeMap<String, u32>,
    #[serde(default)]
    pub buildings: BTreeMap<String, u32>,
    #[serde(default)]
    pub crops: Option<SummaryCrops>,
    pub districts: Vec<SummaryDistrict>,
    #[serde(default)]
    pub food_clusters: Vec<Cluster>,
    #[serde(default)]
    pub tree_clusters: Vec<Cluster>,
    pub time: SummaryTime,
    pub weather: WeatherInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct District {
    pub name: String,
    pub population: PopulationCounts,
    #[serde(default)]
    pub resources: DistrictResources,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoodDistribution {
    pub good: String,
    pub import_option: String,
    pub export_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistrictDistribution {
    pub district: String,
    pub goods: Vec<GoodDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Unlockable {
    pub name: String,
    pub cost: u32,
    pub unlocked: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScienceInfo {
    pub points: u32,
    pub unlockables: Vec<Unlockable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WellbeingNeed {
    pub id: String,
    pub favorable_wellbeing: f64,
    pub unfavorable_wellbeing: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellbeingCategory {
    pub group: String,
    pub current: f64,
    pub max: f64,
    #[serde(default)]
    pub needs: Vec<WellbeingNeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WellbeingReport {
    pub beavers: u32,
    pub categories: Vec<WellbeingCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItem {
    pub subject: String,
    pub description: String,
    pub cycle: u32,
    pub cycle_day: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertItem {
    #[serde(rename = "type")]
    pub alert_type: String,
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrefabCost {
    pub good: String,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrefabInfo {
    pub name: String,
    #[serde(rename = "sizeX")]
    pub size_x: u32,
    #[serde(rename = "sizeY")]
    pub size_y: u32,
    #[serde(rename = "sizeZ")]
    pub size_z: u32,
    #[serde(default)]
    pub science_cost: u32,
    #[serde(default)]
    pub unlocked: bool,
    #[serde(default)]
    pub cost: Vec<PrefabCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PowerBuilding {
    pub id: i64,
    pub name: String,
    pub is_generator: u8,
    pub nominal_input: f64,
    pub nominal_output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerNetwork {
    pub id: i64,
    pub supply: f64,
    pub demand: f64,
    pub buildings: Vec<PowerBuilding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapSize {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TileInfo {
    pub x: i32,
    pub y: i32,
    pub terrain_height: i32,
    #[serde(default)]
    pub water_depth: f64,
    #[serde(default)]
    pub contamination: f64,
    #[serde(default)]
    pub moist: u8,
    #[serde(default)]
    pub occupant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TilesResponse {
    pub map_size: MapSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiles: Option<Vec<TileInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaginatedList<T> {
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    South,
    West,
    North,
    East,
}

/// Verified from `TimberbotWrite.cs:290` and `openapi.yaml:1513` (five levels).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    VeryLow,
    Low,
    Normal,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildingAutomation {
    #[serde(rename = "type", default)]
    pub automation_type: Option<String>,
    #[serde(default)]
    pub has_automator: Option<u8>,
    #[serde(default)]
    pub automator_name: Option<String>,
    #[serde(default)]
    pub is_automated: Option<u8>,
    #[serde(default)]
    pub is_transmitter: Option<u8>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub input_name: Option<String>,
    #[serde(default)]
    pub inputs: Vec<Value>,
    #[serde(default)]
    pub outputs: Vec<Value>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    pub id: i64,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub orientation: Orientation,
    pub finished: u8,
    pub paused: u8,
    #[serde(default)]
    pub pausable: Option<u8>,
    #[serde(default)]
    pub construction_priority: Option<Priority>,
    #[serde(default)]
    pub workplace_priority: Option<Priority>,
    #[serde(default)]
    pub max_workers: Option<u32>,
    #[serde(default)]
    pub desired_workers: Option<u32>,
    #[serde(default)]
    pub assigned_workers: Option<u32>,
    #[serde(default)]
    pub reachable: Option<u8>,
    #[serde(default)]
    pub powered: Option<u8>,
    #[serde(default)]
    pub is_generator: Option<u8>,
    #[serde(default)]
    pub is_consumer: Option<u8>,
    #[serde(default)]
    pub nominal_power_input: Option<f64>,
    #[serde(default)]
    pub nominal_power_output: Option<f64>,
    #[serde(default)]
    pub power_demand: Option<f64>,
    #[serde(default)]
    pub power_supply: Option<f64>,
    #[serde(default)]
    pub build_progress: Option<f64>,
    #[serde(default)]
    pub material_progress: Option<f64>,
    #[serde(default)]
    pub has_materials: Option<u8>,
    #[serde(default)]
    pub stock: Option<i64>,
    #[serde(default)]
    pub capacity: Option<i64>,
    #[serde(default)]
    pub storage_mode: Option<String>,
    #[serde(default)]
    pub allowed_good: Option<String>,
    #[serde(default)]
    pub dwellers: Option<u32>,
    #[serde(default)]
    pub max_dwellers: Option<u32>,
    #[serde(default)]
    pub floodgate: Option<u8>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub max_height: Option<f64>,
    #[serde(default)]
    pub is_clutch: Option<u8>,
    #[serde(default)]
    pub clutch_engaged: Option<u8>,
    #[serde(default)]
    pub current_recipe: Option<String>,
    #[serde(default)]
    pub production_progress: Option<f64>,
    #[serde(default)]
    pub ready_to_produce: Option<u8>,
    #[serde(default)]
    pub alerts: Option<String>,
    #[serde(default)]
    pub inventory: BTreeMap<String, i64>,
    #[serde(default)]
    pub recipes: Vec<String>,
    #[serde(default)]
    pub automation: Option<BuildingAutomation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeaverNeed {
    pub id: String,
    pub group: String,
    pub points: f64,
    pub wellbeing: i32,
    pub favorable: u8,
    pub critical: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Beaver {
    pub id: i64,
    pub name: String,
    pub district: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub is_bot: u8,
    #[serde(default)]
    pub contaminated: Option<u8>,
    #[serde(default)]
    pub deterioration: Option<f64>,
    #[serde(default)]
    pub has_home: Option<u8>,
    #[serde(default)]
    pub life_progress: Option<f64>,
    #[serde(default)]
    pub any_critical: Option<u8>,
    #[serde(default)]
    pub workplace: Option<String>,
    #[serde(default)]
    pub wellbeing: Option<i32>,
    #[serde(default)]
    pub needs: Vec<BeaverNeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plant {
    pub id: i64,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub alive: u8,
    #[serde(default)]
    pub grown: Option<u8>,
    #[serde(default)]
    pub growth: Option<f64>,
    #[serde(default)]
    pub marked: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gatherable {
    pub id: i64,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub alive: u8,
}

// --- Write request bodies matching TimberbotHttpServer.cs + openapi.yaml ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PauseBuildingRequest {
    pub id: i64,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetClutchRequest {
    pub id: i64,
    pub engaged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetFloodgateRequest {
    pub id: i64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetPriorityRequest {
    pub id: i64,
    pub priority: Priority,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub priority_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetHaulingRequest {
    pub id: i64,
    #[serde(default = "default_true")]
    pub prioritized: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetRecipeRequest {
    pub id: i64,
    pub recipe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetFarmhouseRequest {
    pub id: i64,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetPlantableRequest {
    pub id: i64,
    pub plantable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetWorkersRequest {
    pub id: i64,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildingIdRequest {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SetStorageRequest {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub good: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaceBuildingRequest {
    pub prefab: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    #[serde(default = "default_orientation")]
    pub orientation: Orientation,
}

fn default_orientation() -> Orientation {
    Orientation::South
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindPlacementRequest {
    pub prefab: String,
    #[serde(default)]
    pub x1: i32,
    #[serde(default)]
    pub y1: i32,
    #[serde(default)]
    pub x2: i32,
    #[serde(default)]
    pub y2: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlacementCandidate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub orientation: Orientation,
    pub entrance_x: i32,
    pub entrance_y: i32,
    pub path_access: bool,
    pub reachable: bool,
    pub near_power: bool,
    pub flooded: bool,
    pub water_depth: f64,
    pub distance: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindPlacementResponse {
    pub prefab: String,
    pub candidates: Vec<PlacementCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlacePathRequest {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    #[serde(default = "default_path_style")]
    pub style: String,
    #[serde(default)]
    pub sections: u32,
    #[serde(default)]
    pub timings: bool,
}

fn default_path_style() -> String {
    "direct".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlantCropRequest {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub z: i32,
    pub crop: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindPlantingRequest {
    pub crop: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(default)]
    pub x1: i32,
    #[serde(default)]
    pub y1: i32,
    #[serde(default)]
    pub x2: i32,
    #[serde(default)]
    pub y2: i32,
    #[serde(default)]
    pub z: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClearPlantingRequest {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CuttingAreaRequest {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub z: i32,
    #[serde(default = "default_true")]
    pub marked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrateRequest {
    pub from: String,
    pub to: String,
    #[serde(default = "default_one")]
    pub count: u32,
}

fn default_one() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnlockBuildingRequest {
    pub building: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetDistributionRequest {
    pub district: String,
    pub good: String,
    #[serde(rename = "import", default, skip_serializing_if = "Option::is_none")]
    pub import_option: Option<String>,
    #[serde(default = "default_neg_one")]
    pub export_threshold: i32,
}

fn default_neg_one() -> i32 {
    -1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationLinkRequest {
    pub source_id: i64,
    pub target_id: i64,
    #[serde(default = "default_input_a")]
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationUnlinkRequest {
    pub id: i64,
    #[serde(default = "default_input_a")]
    pub input: String,
}

fn default_input_a() -> String {
    "a".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationConfigureRequest {
    pub id: i64,
    pub property: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationRenameRequest {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ListQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub detail: Option<String>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub radius: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TilesQuery {
    pub x1: Option<i32>,
    pub y1: Option<i32>,
    pub x2: Option<i32>,
    pub y2: Option<i32>,
}
