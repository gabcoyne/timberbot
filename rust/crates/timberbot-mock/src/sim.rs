use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use timberbot_protocol::http::*;
use timberbot_protocol::lever::{HttpAdapterState, HttpLeverState};
use timberbot_protocol::ws::{GameEventPayload, ServerFrame};

use crate::data::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HazardKind {
    #[default]
    Drought,
    Badtide,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimMetrics {
    pub cycles_completed: u32,
    pub droughts_survived: u32,
    pub badtides_survived: u32,
    pub deaths: u32,
    pub starvation_events: u32,
    pub thirst_events: u32,
    pub contamination_events: u32,
    pub peak_population: u32,
    pub min_population: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioBuilding {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    #[serde(default = "default_south")]
    pub orientation: Orientation,
    #[serde(default = "default_true")]
    pub finished: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub workers: Option<u32>,
    #[serde(default)]
    pub floodgate_height: Option<f64>,
    #[serde(default)]
    pub max_height: Option<f64>,
    #[serde(default)]
    pub recipe: Option<String>,
    #[serde(default)]
    pub allowed_good: Option<String>,
    #[serde(default)]
    pub storage_mode: Option<String>,
}

fn default_south() -> Orientation {
    Orientation::South
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioConfig {
    pub name: String,
    #[serde(default = "default_faction")]
    pub faction: String,
    #[serde(default)]
    pub difficulty: Difficulty,
    #[serde(default)]
    pub auto_cycle_hazards: bool,
    #[serde(default = "default_district")]
    pub district_name: String,
    #[serde(default = "default_day")]
    pub day_number: u32,
    #[serde(default = "default_cycle")]
    pub cycle: u32,
    #[serde(default = "default_cycle_day")]
    pub cycle_day: u32,
    #[serde(default = "default_temperate")]
    pub temperate_weather_duration: u32,
    #[serde(default = "default_hazardous")]
    pub hazardous_weather_duration: u32,
    #[serde(default)]
    pub is_hazardous: bool,
    #[serde(default)]
    pub hazard_kind: HazardKind,
    #[serde(default = "default_adults")]
    pub adults: u32,
    #[serde(default = "default_children")]
    pub children: u32,
    #[serde(default)]
    pub bots: u32,
    #[serde(default)]
    pub science: u32,
    #[serde(default)]
    pub resources: BTreeMap<String, i64>,
    #[serde(default)]
    pub buildings: Vec<ScenarioBuilding>,
    #[serde(default = "default_trees")]
    pub initial_trees: u32,
    #[serde(default = "default_crops")]
    pub initial_crops: u32,
    #[serde(default = "default_berries")]
    pub initial_gatherables: u32,
    #[serde(default = "default_reservoir")]
    pub reservoir_water_depth: f64,
    #[serde(default)]
    pub river_contamination: f64,
    #[serde(default)]
    pub levers: Vec<HttpLeverState>,
    #[serde(default)]
    pub adapters: Vec<HttpAdapterState>,
}

fn default_faction() -> String {
    "Folktails".to_string()
}
fn default_district() -> String {
    "District 1".to_string()
}
fn default_day() -> u32 {
    1
}
fn default_cycle() -> u32 {
    1
}
fn default_cycle_day() -> u32 {
    1
}
fn default_temperate() -> u32 {
    15
}
fn default_hazardous() -> u32 {
    5
}
fn default_adults() -> u32 {
    9
}
fn default_children() -> u32 {
    4
}
fn default_trees() -> u32 {
    48
}
fn default_crops() -> u32 {
    24
}
fn default_berries() -> u32 {
    30
}
fn default_reservoir() -> f64 {
    1.5
}

impl ScenarioConfig {
    /// Build a verified Timberborn 1.1 starting colony preset for the given faction and difficulty
    /// (`timberborn.wiki.gg/wiki/Game_Mode` starting pop/food/water + handicap curve).
    pub fn preset(faction: &str, difficulty: Difficulty) -> Self {
        let is_it = faction.eq_ignore_ascii_case("IronTeeth");
        let faction_name = if is_it { "IronTeeth" } else { "Folktails" };
        let spec = difficulty.spec();

        let mut resources = BTreeMap::new();
        resources.insert("Water".to_string(), spec.starting_water);
        resources.insert("Berries".to_string(), spec.starting_food);
        resources.insert("Log".to_string(), 60);
        resources.insert("Plank".to_string(), 15);
        resources.insert("Gear".to_string(), 0);

        let dc_name = format!("DistrictCenter.{faction_name}");
        let pump_name = if is_it {
            "DeepWaterPump.IronTeeth"
        } else {
            "WaterPump.Folktails"
        };
        let farm_name = if is_it {
            "FarmHouse.IronTeeth"
        } else {
            "EfficientFarmHouse.Folktails"
        };
        let fg_name = format!("DoubleFloodgate.{faction_name}");
        let lj_name = format!("LumberjackFlag.{faction_name}");
        let gath_name = format!("GathererFlag.{faction_name}");

        let mut buildings = vec![
            ScenarioBuilding {
                name: dc_name,
                x: 16,
                y: 16,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: Some(2),
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            },
            ScenarioBuilding {
                name: pump_name.to_string(),
                x: 12,
                y: 14,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: Some(1),
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            },
            ScenarioBuilding {
                name: farm_name.to_string(),
                x: 20,
                y: 16,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: Some(2),
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            },
            ScenarioBuilding {
                name: lj_name,
                x: 23,
                y: 14,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: Some(1),
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            },
            ScenarioBuilding {
                name: gath_name,
                x: 13,
                y: 20,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: Some(1),
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            },
            ScenarioBuilding {
                name: fg_name,
                x: 10,
                y: 12,
                z: 1,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: None,
                floodgate_height: Some(0.5),
                max_height: Some(2.0),
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            },
        ];

        if is_it {
            buildings.push(ScenarioBuilding {
                name: "Barrack.IronTeeth".to_string(),
                x: 18,
                y: 12,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: None,
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            });
            buildings.push(ScenarioBuilding {
                name: "BreedingPod.IronTeeth".to_string(),
                x: 15,
                y: 13,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: None,
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            });
        } else {
            buildings.push(ScenarioBuilding {
                name: "Lodge.Folktails".to_string(),
                x: 18,
                y: 12,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: None,
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            });
        }

        Self {
            name: format!("{faction_name}-{difficulty:?}"),
            faction: faction_name.to_string(),
            difficulty,
            auto_cycle_hazards: true,
            district_name: default_district(),
            day_number: 1,
            cycle: 1,
            cycle_day: 1,
            temperate_weather_duration: difficulty.nominal_temperate_days(),
            hazardous_weather_duration: difficulty.hazard_duration_for_cycle(1, false),
            is_hazardous: false,
            hazard_kind: HazardKind::Drought,
            adults: spec.starting_adults,
            children: spec.starting_children,
            bots: 0,
            science: 150,
            resources,
            buildings,
            initial_trees: 60,
            initial_crops: 36,
            initial_gatherables: 32,
            reservoir_water_depth: 1.6,
            river_contamination: 0.0,
            levers: Vec::new(),
            adapters: Vec::new(),
        }
    }
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        let mut resources = BTreeMap::new();
        resources.insert("Water".to_string(), 80);
        resources.insert("Berries".to_string(), 60);
        resources.insert("Carrot".to_string(), 40);
        resources.insert("Log".to_string(), 45);
        resources.insert("Plank".to_string(), 10);

        Self {
            name: "MockColony".to_string(),
            faction: default_faction(),
            difficulty: Difficulty::Normal,
            auto_cycle_hazards: false,
            district_name: default_district(),
            day_number: 1,
            cycle: 1,
            cycle_day: 1,
            temperate_weather_duration: 13,
            hazardous_weather_duration: 4,
            is_hazardous: false,
            hazard_kind: HazardKind::Drought,
            adults: 9,
            children: 4,
            bots: 0,
            science: 150,
            resources,
            buildings: vec![
                ScenarioBuilding {
                    name: "DistrictCenter.Folktails".to_string(),
                    x: 16,
                    y: 16,
                    z: 2,
                    orientation: Orientation::South,
                    finished: true,
                    paused: false,
                    workers: Some(2),
                    floodgate_height: None,
                    max_height: None,
                    recipe: None,
                    allowed_good: None,
                    storage_mode: None,
                },
                ScenarioBuilding {
                    name: "WaterPump.Folktails".to_string(),
                    x: 12,
                    y: 14,
                    z: 2,
                    orientation: Orientation::South,
                    finished: true,
                    paused: false,
                    workers: Some(1),
                    floodgate_height: None,
                    max_height: None,
                    recipe: None,
                    allowed_good: None,
                    storage_mode: None,
                },
                ScenarioBuilding {
                    name: "EfficientFarmHouse.Folktails".to_string(),
                    x: 20,
                    y: 16,
                    z: 2,
                    orientation: Orientation::South,
                    finished: true,
                    paused: false,
                    workers: Some(2),
                    floodgate_height: None,
                    max_height: None,
                    recipe: None,
                    allowed_good: None,
                    storage_mode: None,
                },
                ScenarioBuilding {
                    name: "LumberjackFlag.Folktails".to_string(),
                    x: 23,
                    y: 14,
                    z: 2,
                    orientation: Orientation::South,
                    finished: true,
                    paused: false,
                    workers: Some(1),
                    floodgate_height: None,
                    max_height: None,
                    recipe: None,
                    allowed_good: None,
                    storage_mode: None,
                },
                ScenarioBuilding {
                    name: "GathererFlag.Folktails".to_string(),
                    x: 13,
                    y: 20,
                    z: 2,
                    orientation: Orientation::South,
                    finished: true,
                    paused: false,
                    workers: Some(1),
                    floodgate_height: None,
                    max_height: None,
                    recipe: None,
                    allowed_good: None,
                    storage_mode: None,
                },
                ScenarioBuilding {
                    name: "Lodge.Folktails".to_string(),
                    x: 18,
                    y: 12,
                    z: 2,
                    orientation: Orientation::South,
                    finished: true,
                    paused: false,
                    workers: None,
                    floodgate_height: None,
                    max_height: None,
                    recipe: None,
                    allowed_good: None,
                    storage_mode: None,
                },
                ScenarioBuilding {
                    name: "DoubleFloodgate.Folktails".to_string(),
                    x: 10,
                    y: 12,
                    z: 1,
                    orientation: Orientation::South,
                    finished: true,
                    paused: false,
                    workers: None,
                    floodgate_height: Some(0.5),
                    max_height: Some(2.0),
                    recipe: None,
                    allowed_good: None,
                    storage_mode: None,
                },
            ],
            initial_trees: 48,
            initial_crops: 24,
            initial_gatherables: 30,
            reservoir_water_depth: 1.5,
            river_contamination: 0.0,
            levers: vec![HttpLeverState {
                name: "MainGate".to_string(),
                state: false,
                spring_return: false,
                color: Some("00FF00".to_string()),
            }],
            adapters: vec![HttpAdapterState {
                name: "LowWaterSensor".to_string(),
                state: false,
            }],
        }
    }
}

pub struct MockWorld {
    pub settlement: String,
    pub faction: String,
    pub difficulty: Difficulty,
    pub auto_cycle_hazards: bool,
    pub district_name: String,
    pub map_size: MapSize,

    pub day_number: u32,
    pub day_progress: f64,
    pub speed: u8,
    pub work_end_hours: u8,

    pub cycle: u32,
    pub cycle_day: u32,
    pub temperate_weather_duration: u32,
    pub hazardous_weather_duration: u32,
    pub is_hazardous: bool,
    pub hazard_kind: HazardKind,
    approaching_emitted_for_cycle: u32,

    pub reservoir_water_depth: f64,
    pub river_contamination: f64,

    pub science: u32,
    pub science_accum: f64,
    pub resources: BTreeMap<String, f64>,
    pub buildings: Vec<Building>,
    pub beavers: Vec<Beaver>,
    pub trees: Vec<Plant>,
    pub crops: Vec<Plant>,
    pub gatherables: Vec<Gatherable>,
    pub prefabs: Vec<PrefabInfo>,
    pub unlockables: Vec<Unlockable>,
    pub notifications: Vec<NotificationItem>,
    pub distribution: Vec<DistrictDistribution>,
    pub agent_messages: Vec<String>,

    pub levers: Vec<HttpLeverState>,
    pub adapters: Vec<HttpAdapterState>,

    pub agent_state: AgentState,
    pub last_acked_request_id: i64,
    pub metrics: SimMetrics,
    next_request_id: i64,
    next_entity_id: i64,
    dehydration_stress: f64,
    starvation_stress: f64,
    contamination_exposure: f64,
    was_starving: bool,
    was_thirsty: bool,
    was_contaminated: bool,
    breeding_accum: f64,
}

impl MockWorld {
    pub fn from_scenario(cfg: ScenarioConfig) -> Self {
        let mut next_entity_id = 1000_i64;
        let mut alloc_id = || {
            let id = next_entity_id;
            next_entity_id += 1;
            id
        };

        let is_it = cfg.faction.eq_ignore_ascii_case("IronTeeth");
        let prefabs = prefabs_for_faction(&cfg.faction);
        let unlockables = prefabs
            .iter()
            .filter(|p| p.science_cost > 0)
            .map(|p| Unlockable {
                name: p.name.clone(),
                cost: p.science_cost,
                unlocked: if p.unlocked { 1 } else { 0 },
            })
            .collect();

        let mut buildings = Vec::new();
        for sb in &cfg.buildings {
            buildings.push(make_building(alloc_id(), sb));
        }

        let mut beavers = Vec::new();
        for i in 0..cfg.adults {
            beavers.push(make_beaver(
                alloc_id(),
                format!("Beaver-{}", i + 1),
                &cfg.district_name,
                false,
                0.35,
            ));
        }
        for i in 0..cfg.children {
            beavers.push(make_beaver(
                alloc_id(),
                format!("Kit-{}", i + 1),
                &cfg.district_name,
                false,
                0.08,
            ));
        }
        for i in 0..cfg.bots {
            beavers.push(make_beaver(
                alloc_id(),
                format!("Bot-{}", i + 1),
                &cfg.district_name,
                true,
                0.2,
            ));
        }

        let mut trees = Vec::new();
        for i in 0..cfg.initial_trees {
            trees.push(Plant {
                id: alloc_id(),
                name: if i % 3 == 0 {
                    "Birch".to_string()
                } else {
                    "Pine".to_string()
                },
                x: 24 + (i % 8) as i32,
                y: 10 + (i / 8) as i32,
                z: 2,
                alive: 1,
                grown: Some(1),
                growth: Some(1.0),
                marked: Some(if i < 16 { 1 } else { 0 }),
            });
        }

        let starter_crop = if is_it { "Kohlrabi" } else { "Carrot" };
        let mut crops = Vec::new();
        for i in 0..cfg.initial_crops {
            crops.push(Plant {
                id: alloc_id(),
                name: starter_crop.to_string(),
                x: 19 + (i % 6) as i32,
                y: 18 + (i / 6) as i32,
                z: 2,
                alive: 1,
                grown: Some(if i % 2 == 0 { 1 } else { 0 }),
                growth: Some(if i % 2 == 0 { 1.0 } else { 0.6 }),
                marked: Some(0),
            });
        }

        let mut gatherables = Vec::new();
        for i in 0..cfg.initial_gatherables {
            gatherables.push(Gatherable {
                id: alloc_id(),
                name: "BlueberryBush".to_string(),
                x: 8 + (i % 6) as i32,
                y: 22 + (i / 6) as i32,
                z: 2,
                alive: 1,
            });
        }

        let mut resources = BTreeMap::new();
        for (k, v) in &cfg.resources {
            resources.insert(k.clone(), (*v).max(0) as f64);
        }
        for key in ["Water", "Berries", starter_crop, "Log", "Plank"] {
            resources.entry(key.to_string()).or_insert(0.0);
        }

        let distribution = vec![DistrictDistribution {
            district: cfg.district_name.clone(),
            goods: vec![
                GoodDistribution {
                    good: "Water".to_string(),
                    import_option: "Auto".to_string(),
                    export_threshold: 0.0,
                },
                GoodDistribution {
                    good: "Log".to_string(),
                    import_option: "Auto".to_string(),
                    export_threshold: 0.0,
                },
            ],
        }];

        let init_pop = beavers.len() as u32;
        let mut world = Self {
            settlement: cfg.name,
            faction: cfg.faction,
            difficulty: cfg.difficulty,
            auto_cycle_hazards: cfg.auto_cycle_hazards,
            district_name: cfg.district_name,
            map_size: MapSize { x: 32, y: 32, z: 8 },
            day_number: cfg.day_number,
            day_progress: 0.25,
            speed: 1,
            work_end_hours: 16,
            cycle: cfg.cycle,
            cycle_day: cfg.cycle_day,
            temperate_weather_duration: cfg.temperate_weather_duration.max(1),
            hazardous_weather_duration: cfg.hazardous_weather_duration.max(1),
            is_hazardous: cfg.is_hazardous,
            hazard_kind: cfg.hazard_kind,
            approaching_emitted_for_cycle: 0,
            reservoir_water_depth: cfg.reservoir_water_depth,
            river_contamination: cfg.river_contamination,
            science: cfg.science,
            science_accum: 0.0,
            resources,
            buildings,
            beavers,
            trees,
            crops,
            gatherables,
            prefabs,
            unlockables,
            notifications: Vec::new(),
            distribution,
            agent_messages: Vec::new(),
            levers: cfg.levers,
            adapters: cfg.adapters,
            agent_state: AgentState {
                mode: AgentMode::Request,
                goal: String::new(),
                ready: false,
                pending_request: None,
                agent_status: "idle".to_string(),
                last_error: None,
            },
            last_acked_request_id: 0,
            metrics: SimMetrics {
                peak_population: init_pop,
                min_population: init_pop,
                ..Default::default()
            },
            next_request_id: 1,
            next_entity_id,
            dehydration_stress: 0.0,
            starvation_stress: 0.0,
            contamination_exposure: 0.0,
            was_starving: false,
            was_thirsty: false,
            was_contaminated: false,
            breeding_accum: 0.0,
        };
        world.rebalance_workers_and_housing();
        world
    }

    pub fn alloc_id(&mut self) -> i64 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    pub fn enqueue_request(&mut self, prompt: String) -> PendingRequest {
        let id = self.next_request_id;
        self.next_request_id += 1;
        let req = PendingRequest { id, prompt };
        self.agent_state.pending_request = Some(req.clone());
        req
    }

    pub fn apply_heartbeat(&mut self, agent_status: &str, acked_request_id: i64) {
        if !agent_status.trim().is_empty() {
            self.agent_state.agent_status = agent_status.trim().to_string();
        }
        if acked_request_id > self.last_acked_request_id {
            self.last_acked_request_id = acked_request_id;
        }
        if let Some(pending) = &self.agent_state.pending_request {
            if acked_request_id >= pending.id {
                self.agent_state.pending_request = None;
            }
        }
    }

    pub fn cycle_length(&self) -> u32 {
        self.temperate_weather_duration + self.hazardous_weather_duration
    }

    pub fn days_until_hazard(&self) -> i32 {
        if self.is_hazardous {
            0
        } else {
            self.temperate_weather_duration as i32 - self.cycle_day as i32 + 1
        }
    }

    fn is_iron_teeth(&self) -> bool {
        self.faction.eq_ignore_ascii_case("IronTeeth")
    }

    fn choose_next_cycle_weather(&mut self) {
        if !self.auto_cycle_hazards {
            return;
        }
        let spec = self.difficulty.spec();
        // Deterministic badtide schedule once past badtide_min_cycle: alternate badtide on
        // qualifying cycles matching the ~40% probability without randomness.
        let is_badtide = self.cycle >= spec.badtide_min_cycle
            && (self.cycle - spec.badtide_min_cycle).is_multiple_of(2);
        self.hazard_kind = if is_badtide {
            HazardKind::Badtide
        } else {
            HazardKind::Drought
        };
        self.temperate_weather_duration = self.difficulty.nominal_temperate_days();
        self.hazardous_weather_duration = self
            .difficulty
            .hazard_duration_for_cycle(self.cycle, is_badtide);
    }

    /// Advance the colony simulation deterministically using verified Timberborn 1.1 rates.
    /// Does nothing when `speed == 0` (paused).
    pub fn step(&mut self, base_days: f64) -> Vec<ServerFrame> {
        if self.speed == 0 || base_days <= 0.0 {
            return Vec::new();
        }
        let dt = base_days * (self.speed as f64);
        let mut events = Vec::new();
        let now = unix_now();

        let prev_day_progress = self.day_progress;
        self.day_progress += dt;

        // Night start crossing
        let night_mark = (self.work_end_hours as f64) / 24.0;
        if prev_day_progress < night_mark && self.day_progress >= night_mark {
            events.push(make_event(
                "night.start",
                self.day_number,
                now,
                Some(json!({ "day": self.day_number })),
            ));
        }

        // Advance day boundaries
        while self.day_progress >= 1.0 {
            self.day_progress -= 1.0;
            self.day_number += 1;
            self.cycle_day += 1;

            events.push(make_event(
                "day.start",
                self.day_number,
                now,
                Some(json!({ "day": self.day_number })),
            ));

            // Age kits toward adulthood (5 days to grow into adult at life_progress >= 0.2)
            for b in &mut self.beavers {
                if b.is_bot == 0 {
                    let lp = b.life_progress.unwrap_or(0.35);
                    if lp < 0.2 {
                        b.life_progress = Some((lp + 0.04).min(0.25));
                    }
                }
            }

            // Drought / badtide approaching warning (within 3 days of hazard start)
            if !self.is_hazardous
                && self.days_until_hazard() <= 3
                && self.days_until_hazard() > 0
                && self.approaching_emitted_for_cycle != self.cycle
            {
                self.approaching_emitted_for_cycle = self.cycle;
                events.push(make_event(
                    "drought.approaching",
                    self.day_number,
                    now,
                    None,
                ));
                self.notifications.push(NotificationItem {
                    subject: match self.hazard_kind {
                        HazardKind::Drought => "DroughtApproaching".to_string(),
                        HazardKind::Badtide => "BadtideApproaching".to_string(),
                    },
                    description: format!(
                        "{} begins in {} days",
                        match self.hazard_kind {
                            HazardKind::Drought => "Drought",
                            HazardKind::Badtide => "Badtide",
                        },
                        self.days_until_hazard()
                    ),
                    cycle: self.cycle,
                    cycle_day: self.cycle_day,
                    entity_id: None,
                });
            }

            // Transition temperate -> hazardous
            if !self.is_hazardous && self.cycle_day > self.temperate_weather_duration {
                self.is_hazardous = true;
                events.push(make_event(
                    "drought.start",
                    self.day_number,
                    now,
                    Some(json!({
                        "duration": self.hazardous_weather_duration,
                        "kind": match self.hazard_kind {
                            HazardKind::Drought => "drought",
                            HazardKind::Badtide => "badtide",
                        }
                    })),
                ));
                self.notifications.push(NotificationItem {
                    subject: match self.hazard_kind {
                        HazardKind::Drought => "DroughtStarted".to_string(),
                        HazardKind::Badtide => "BadtideStarted".to_string(),
                    },
                    description: format!(
                        "{} started for {} days",
                        match self.hazard_kind {
                            HazardKind::Drought => "Drought",
                            HazardKind::Badtide => "Badtide",
                        },
                        self.hazardous_weather_duration
                    ),
                    cycle: self.cycle,
                    cycle_day: self.cycle_day,
                    entity_id: None,
                });
            }

            // Transition hazardous -> next temperate cycle
            if self.is_hazardous && self.cycle_day > self.cycle_length() {
                let survived_kind = self.hazard_kind;
                self.is_hazardous = false;
                if !self.beavers.is_empty() {
                    self.metrics.cycles_completed += 1;
                    match survived_kind {
                        HazardKind::Drought => self.metrics.droughts_survived += 1,
                        HazardKind::Badtide => self.metrics.badtides_survived += 1,
                    }
                }
                events.push(make_event("drought.end", self.day_number, now, None));
                events.push(make_event(
                    "cycle.end",
                    self.day_number,
                    now,
                    Some(json!({ "cycle": self.cycle })),
                ));
                self.cycle += 1;
                self.cycle_day = 1;
                self.choose_next_cycle_weather();
                events.push(make_event(
                    "cycle.start",
                    self.day_number,
                    now,
                    Some(json!({ "cycle": self.cycle })),
                ));
            }
        }

        // Update river water depth & contamination using verified Dam/Floodgate & Evaporation rates
        let max_gate_height = self
            .buildings
            .iter()
            .filter(|b| b.finished == 1 && b.floodgate == Some(1))
            .filter_map(|b| b.height)
            .fold(0.0_f64, f64::max);
        let has_dam = self
            .buildings
            .iter()
            .any(|b| b.finished == 1 && b.name.starts_with("Dam."));

        if self.is_hazardous {
            match self.hazard_kind {
                HazardKind::Drought => {
                    self.river_contamination = 0.0;
                    let evap = if max_gate_height >= DAM_SPILL_HEIGHT || has_dam {
                        EVAP_WIDE_POOL_PER_DAY
                    } else if max_gate_height > 0.0 {
                        EVAP_MEDIUM_CHANNEL_PER_DAY
                    } else {
                        EVAP_NARROW_CHANNEL_PER_DAY
                    };
                    self.reservoir_water_depth = (self.reservoir_water_depth - evap * dt).max(0.0);
                }
                HazardKind::Badtide => {
                    // Verified direction: raising a floodgate blocks incoming badwater flow
                    // (diverting it upstream); height 0 is wide open and floods the reservoir.
                    let diverted = max_gate_height >= 0.8;
                    if diverted {
                        self.river_contamination = (self.river_contamination - 0.5 * dt).max(0.0);
                        self.reservoir_water_depth =
                            (self.reservoir_water_depth - EVAP_WIDE_POOL_PER_DAY * dt).max(0.0);
                    } else {
                        self.river_contamination = 0.95;
                        self.reservoir_water_depth = 1.5;
                    }
                }
            }
        } else {
            self.river_contamination = (self.river_contamination - 0.5 * dt).max(0.0);
            let target_depth = if max_gate_height >= 0.8 {
                (max_gate_height + 0.2).min(2.2)
            } else if has_dam {
                1.6
            } else {
                1.4
            };
            self.reservoir_water_depth = (self.reservoir_water_depth + 0.6 * dt).min(target_depth);
        }

        // Track badwater contamination exposure (`CONTAMINATION_ONSET_DAYS = 3.0d`)
        let exposed_to_badwater = self.river_contamination >= IRRIGATION_POLLUTION_CUTOFF;
        if exposed_to_badwater {
            self.contamination_exposure += dt;
        } else {
            self.contamination_exposure = (self.contamination_exposure - 0.5 * dt).max(0.0);
        }
        let colony_contaminated = self.contamination_exposure >= CONTAMINATION_ONSET_DAYS;
        if colony_contaminated && !self.was_contaminated {
            self.metrics.contamination_events += 1;
        }
        self.was_contaminated = colony_contaminated;

        // Rebalance worker assignment & housing (contaminated beavers refuse work!)
        self.rebalance_workers_and_housing();
        self.update_power_grid();

        // Advance construction sites
        let mut newly_finished = Vec::new();
        for b in &mut self.buildings {
            if b.finished == 0 && b.paused == 0 {
                let prev = b.build_progress.unwrap_or(0.0);
                let next = (prev + 1.2 * dt).min(1.0);
                b.build_progress = Some(next);
                b.material_progress = Some(1.0);
                if next >= 1.0 {
                    b.finished = 1;
                    newly_finished.push((b.id, b.name.clone()));
                }
            }
        }
        for (id, name) in newly_finished {
            events.push(make_event(
                "building.finished",
                self.day_number,
                now,
                Some(json!({ "id": id, "name": name })),
            ));
        }

        // Production scaled by verified work hours (hours/day * dt)
        let work_hours_dt = (self.work_end_hours as f64) * dt;
        let mut water_cap = 300.0_f64; // District Center base storage
        let mut water_prod = 0.0_f64;
        let mut crop_harvest_cap = 0.0_f64;
        let mut berry_prod = 0.0_f64;
        let mut log_prod = 0.0_f64;
        let mut plank_prod = 0.0_f64;
        let mut science_prod = 0.0_f64;
        let mut fermented_prod = 0.0_f64;

        let irrigated = self.reservoir_water_depth > 0.05
            && self.river_contamination < IRRIGATION_POLLUTION_CUTOFF;
        let primary_crop = if self.is_iron_teeth() {
            "Kohlrabi"
        } else {
            "Carrot"
        };

        for b in &self.buildings {
            if b.finished == 0 {
                continue;
            }
            if b.name.contains("Tank") {
                water_cap += b.capacity.unwrap_or(30) as f64;
            }
            if b.paused == 1 {
                continue;
            }
            let workers = b.assigned_workers.unwrap_or(0) as f64;
            if workers <= 0.0 {
                continue;
            }
            if b.name.contains("LargeWaterPump") {
                if self.reservoir_water_depth > 0.05
                    && self.river_contamination < IRRIGATION_POLLUTION_CUTOFF
                {
                    water_prod += workers * LARGE_WATER_PUMP_PER_HOUR * work_hours_dt;
                }
            } else if b.name.contains("DeepWaterPump") {
                if self.reservoir_water_depth > 0.02
                    && self.river_contamination < IRRIGATION_POLLUTION_CUTOFF
                {
                    water_prod += workers * WATER_PUMP_PER_HOUR * work_hours_dt;
                }
            } else if b.name.contains("WaterPump") {
                if self.reservoir_water_depth > 0.05
                    && self.river_contamination < IRRIGATION_POLLUTION_CUTOFF
                {
                    water_prod += workers * WATER_PUMP_PER_HOUR * work_hours_dt;
                }
            } else if b.name.contains("FarmHouse") || b.name.contains("Farmhouse") {
                // Farmers can harvest up to ~2.5 crop units/hr each; total food is bounded
                // by actual grown crop tiles (`crop_spec`: Carrot 3/4d, Kohlrabi 2/3d).
                crop_harvest_cap += workers * 2.5 * work_hours_dt;
            } else if b.name.contains("GathererFlag") && !self.gatherables.is_empty() {
                let bush_factor = (self.gatherables.len() as f64 / 12.0).clamp(0.25, 1.0);
                berry_prod += workers * 2.4 * bush_factor * work_hours_dt;
            } else if b.name.contains("LumberjackFlag") {
                let marked = self
                    .trees
                    .iter()
                    .filter(|t| t.alive == 1 && t.marked == Some(1))
                    .count();
                if marked > 0 {
                    log_prod += workers * 0.75 * work_hours_dt;
                }
            } else if b.name.contains("Inventor") {
                science_prod += workers * INVENTOR_SP_PER_HOUR * work_hours_dt;
            } else if b.name.contains("LumberMill") && b.powered == Some(1) {
                plank_prod += workers * LUMBER_MILL_PLANKS_PER_HOUR * work_hours_dt;
            } else if b.name.contains("Fermenter") && b.powered == Some(1) {
                fermented_prod += workers * FERMENTER_OUTPUT_PER_HOUR * work_hours_dt;
            }
        }

        // Grow and harvest crops using verified growth days & yields (`crop_spec`)
        let mut harvested_food = 0.0_f64;
        for c in &mut self.crops {
            if c.alive != 1 {
                continue;
            }
            let (grow_days, yield_amt) = crop_spec(&c.name);
            let mut g = c.growth.unwrap_or(0.0);
            if irrigated && g < 1.0 {
                g = (g + dt / grow_days).min(1.0);
                c.growth = Some(g);
                c.grown = Some(if g >= 1.0 { 1 } else { 0 });
            }
            if g >= 1.0 && crop_harvest_cap >= yield_amt {
                crop_harvest_cap -= yield_amt;
                harvested_food += yield_amt;
                // Automatically replanted if irrigated
                if irrigated {
                    c.growth = Some(0.0);
                    c.grown = Some(0);
                } else {
                    c.growth = Some(0.1);
                    c.grown = Some(0);
                }
            }
        }

        // Apply production (cap pumped water at storage capacity without erasing existing stock)
        if water_prod > 0.0 {
            let cur_water = *self.resources.get("Water").unwrap_or(&0.0);
            self.resources.insert(
                "Water".to_string(),
                (cur_water + water_prod).min(water_cap.max(cur_water)),
            );
        }
        *self
            .resources
            .entry(primary_crop.to_string())
            .or_insert(0.0) += harvested_food;
        *self.resources.entry("Berries".to_string()).or_insert(0.0) += berry_prod;
        *self.resources.entry("Log".to_string()).or_insert(0.0) += log_prod;

        // Lumber Mill converts 1 Log -> 1 Plank
        if plank_prod > 0.0 {
            let logs_avail = *self.resources.get("Log").unwrap_or(&0.0);
            let actual_planks = plank_prod.min(logs_avail);
            *self.resources.entry("Log".to_string()).or_insert(0.0) -= actual_planks;
            *self.resources.entry("Plank".to_string()).or_insert(0.0) += actual_planks;
        }

        // Fermenter converts 4 Cassava -> 10 FermentedCassava
        if fermented_prod > 0.0 {
            let cassava_avail = *self.resources.get("Cassava").unwrap_or(&0.0);
            let cassava_needed = fermented_prod * 0.4;
            let actual_cassava = cassava_needed.min(cassava_avail);
            if actual_cassava > 0.0 {
                *self.resources.entry("Cassava".to_string()).or_insert(0.0) -= actual_cassava;
                *self
                    .resources
                    .entry("FermentedCassava".to_string())
                    .or_insert(0.0) += actual_cassava * 2.5;
            }
        }

        self.science_accum += science_prod;
        let whole_sp = self.science_accum.floor() as u32;
        if whole_sp > 0 {
            self.science_accum -= whole_sp as f64;
            self.science = self.science.saturating_add(whole_sp);
        }

        // Beaver consumption using verified Timberborn 1.1 rates * difficulty multiplier
        let cons_mult = self.difficulty.spec().consumption_mult;
        let organic_beavers = self.beavers.iter().filter(|b| b.is_bot == 0).count() as f64;
        let water_need = organic_beavers * WATER_PER_BEAVER_DAY * cons_mult * dt;
        let food_need = organic_beavers * FOOD_PER_BEAVER_DAY * cons_mult * dt;

        let water_after = *self.resources.get("Water").unwrap_or(&0.0) - water_need;
        let is_thirsty = water_after < 0.0;
        if is_thirsty {
            self.resources.insert("Water".to_string(), 0.0);
            self.dehydration_stress += dt;
            if !self.was_thirsty {
                self.metrics.thirst_events += 1;
            }
        } else {
            self.resources.insert("Water".to_string(), water_after);
            self.dehydration_stress = (self.dehydration_stress - dt).max(0.0);
        }
        self.was_thirsty = is_thirsty;

        let mut remaining_food_need = food_need;
        for food_key in [
            "Carrot",
            "Kohlrabi",
            "FermentedCassava",
            "GrilledPotato",
            "Bread",
            "Berries",
        ] {
            if remaining_food_need <= 0.0 {
                break;
            }
            if let Some(stock) = self.resources.get_mut(food_key) {
                let take = stock.min(remaining_food_need);
                *stock -= take;
                remaining_food_need -= take;
            }
        }
        let is_starving = remaining_food_need > 0.01;
        if is_starving {
            self.starvation_stress += dt;
            if !self.was_starving {
                self.metrics.starvation_events += 1;
            }
        } else {
            self.starvation_stress = (self.starvation_stress - dt).max(0.0);
        }
        self.was_starving = is_starving;

        // Verified death thresholds: 3.75d starving, 4.29d thirsty
        if self.dehydration_stress >= DEHYDRATION_DEATH_DAYS
            || self.starvation_stress >= STARVATION_DEATH_DAYS
        {
            if let Some(pos) = self.beavers.iter().position(|b| b.is_bot == 0) {
                self.beavers.remove(pos);
                self.metrics.deaths += 1;
                self.dehydration_stress = (self.dehydration_stress - 1.5).max(0.0);
                self.starvation_stress = (self.starvation_stress - 1.5).max(0.0);
                events.push(make_event("character.killed", self.day_number, now, None));
                events.push(make_event("population.changed", self.day_number, now, None));
            }
        }

        // Faction-accurate reproduction
        if !is_starving && !is_thirsty && !self.beavers.is_empty() {
            if self.is_iron_teeth() {
                // Iron Teeth: each active BreedingPod consumes 5 Water + 5 Berries over 5 days
                let active_pods = self
                    .buildings
                    .iter()
                    .filter(|b| {
                        b.finished == 1 && b.paused == 0 && b.name.starts_with("BreedingPod")
                    })
                    .count() as f64;
                if active_pods > 0.0 {
                    let water_avail = *self.resources.get("Water").unwrap_or(&0.0);
                    let berries_avail = *self.resources.get("Berries").unwrap_or(&0.0);
                    let w_tick =
                        active_pods * (BREEDING_POD_WATER_COST / BREEDING_POD_CYCLE_DAYS) * dt;
                    let b_tick =
                        active_pods * (BREEDING_POD_BERRIES_COST / BREEDING_POD_CYCLE_DAYS) * dt;
                    if water_avail >= w_tick && berries_avail >= b_tick {
                        *self.resources.entry("Water".to_string()).or_insert(0.0) -= w_tick;
                        *self.resources.entry("Berries".to_string()).or_insert(0.0) -= b_tick;
                        self.breeding_accum += active_pods * (dt / BREEDING_POD_CYCLE_DAYS);
                    }
                }
            } else {
                // Folktails: breed when free beds exist in finished housing
                let total_beds: u32 = self
                    .buildings
                    .iter()
                    .filter(|b| b.finished == 1)
                    .filter_map(|b| b.max_dwellers)
                    .sum();
                let organic = self.beavers.iter().filter(|b| b.is_bot == 0).count() as u32;
                if organic < total_beds && self.beavers.len() < 40 {
                    let free_beds = (total_beds - organic) as f64;
                    self.breeding_accum += (free_beds.min(3.0) * 0.15) * dt;
                }
            }

            if self.breeding_accum >= 1.0 && self.beavers.len() < 40 {
                self.breeding_accum -= 1.0;
                let kit_id = self.alloc_id();
                let kit_num = self.beavers.len() + 1;
                let dist = self.district_name.clone();
                self.beavers.push(make_beaver(
                    kit_id,
                    format!("Kit-{kit_num}"),
                    &dist,
                    false,
                    0.0,
                ));
                events.push(make_event("population.changed", self.day_number, now, None));
            }
        }

        // Update beaver status flags & adapter signals
        let low_water =
            *self.resources.get("Water").unwrap_or(&0.0) < organic_beavers * WATER_PER_BEAVER_DAY;
        let low_food = self.total_food() < (organic_beavers * FOOD_PER_BEAVER_DAY) as i64;
        for b in &mut self.beavers {
            let crit = if b.is_bot == 0 && (low_water || low_food || colony_contaminated) {
                1
            } else {
                0
            };
            b.any_critical = Some(crit);
            b.contaminated = Some(if colony_contaminated && b.is_bot == 0 {
                1
            } else {
                0
            });
            let mut wb = 8.0;
            if b.has_home == Some(0) {
                wb -= 3.0;
            }
            if is_starving {
                wb -= 10.0;
            }
            if is_thirsty {
                wb -= 10.0;
            }
            if colony_contaminated {
                wb -= 10.0;
            }
            b.wellbeing = Some(wb);
        }

        let cur_pop = self.beavers.len() as u32;
        if cur_pop > self.metrics.peak_population {
            self.metrics.peak_population = cur_pop;
        }
        if cur_pop < self.metrics.min_population {
            self.metrics.min_population = cur_pop;
        }

        for adapter in &mut self.adapters {
            if adapter.name.to_ascii_lowercase().contains("water") {
                adapter.state = low_water || self.is_hazardous;
            }
        }

        events
    }

    pub fn update_power_grid(&mut self) {
        let mut total_supply = 0.0_f64;
        let mut total_demand = 0.0_f64;
        let flowing = !self.is_hazardous && self.reservoir_water_depth > 0.2;

        for b in &mut self.buildings {
            if b.finished == 0 || b.paused == 1 {
                b.power_supply = Some(0.0);
                b.power_demand = Some(0.0);
                continue;
            }
            let nom_out = b.nominal_power_output.unwrap_or(0.0);
            let nom_in = b.nominal_power_input.unwrap_or(0.0);
            if nom_out > 0.0 {
                let actual = if b.name.contains("WaterWheel") {
                    if flowing {
                        nom_out
                    } else {
                        0.0
                    }
                } else if b.name.contains("PowerWheel") {
                    let max_w = b.max_workers.unwrap_or(1).max(1) as f64;
                    let w = b.assigned_workers.unwrap_or(0) as f64;
                    nom_out * (w / max_w)
                } else {
                    nom_out
                };
                b.power_supply = Some(actual);
                total_supply += actual;
            }
            if nom_in > 0.0 {
                b.power_demand = Some(nom_in);
                total_demand += nom_in;
            }
        }

        let grid_ok = total_demand <= 0.0 || total_supply >= total_demand;
        for b in &mut self.buildings {
            if b.nominal_power_input.unwrap_or(0.0) > 0.0 {
                b.powered = Some(if grid_ok { 1 } else { 0 });
            } else {
                b.powered = Some(1);
            }
        }
    }

    pub fn power_networks(&self) -> Vec<PowerNetwork> {
        let mut supply = 0.0_f64;
        let mut demand = 0.0_f64;
        let mut buildings = Vec::new();
        for b in &self.buildings {
            if b.finished == 1 && (b.is_generator == Some(1) || b.is_consumer == Some(1)) {
                supply += b.power_supply.unwrap_or(0.0);
                demand += b.power_demand.unwrap_or(0.0);
                buildings.push(PowerBuilding {
                    id: b.id,
                    name: b.name.clone(),
                    is_generator: b.is_generator.unwrap_or(0),
                    nominal_input: b.nominal_power_input.unwrap_or(0.0),
                    nominal_output: b.nominal_power_output.unwrap_or(0.0),
                });
            }
        }
        if buildings.is_empty() {
            return Vec::new();
        }
        vec![PowerNetwork {
            id: 1,
            supply,
            demand,
            buildings,
        }]
    }

    pub fn rebalance_workers_and_housing(&mut self) {
        // Contaminated beavers refuse work (verified: `timberborn.wiki.gg/wiki/Needs`)
        let total_adults = self
            .beavers
            .iter()
            .filter(|b| {
                b.is_bot == 0
                    && b.life_progress.unwrap_or(0.5) >= 0.2
                    && b.contaminated.unwrap_or(0) == 0
            })
            .count() as u32;
        let mut available = total_adults;

        let mut indices: Vec<usize> = (0..self.buildings.len()).collect();
        indices.sort_by_key(|&i| {
            let p = self.buildings[i]
                .workplace_priority
                .unwrap_or(Priority::Normal);
            std::cmp::Reverse(priority_rank(p))
        });

        for i in indices {
            let b = &mut self.buildings[i];
            if b.finished == 0 || b.paused == 1 {
                b.assigned_workers = Some(0);
                continue;
            }
            let desired = b
                .desired_workers
                .unwrap_or(0)
                .min(b.max_workers.unwrap_or(0));
            let assign = desired.min(available);
            b.assigned_workers = Some(assign);
            available = available.saturating_sub(assign);
        }

        let total_beavers = self.beavers.iter().filter(|b| b.is_bot == 0).count() as u32;
        let mut remaining_beavers = total_beavers;
        for b in &mut self.buildings {
            let max_d = b.max_dwellers.unwrap_or(0);
            if b.finished == 1 && max_d > 0 {
                let occ = max_d.min(remaining_beavers);
                b.dwellers = Some(occ);
                remaining_beavers = remaining_beavers.saturating_sub(occ);
            } else {
                b.dwellers = Some(0);
            }
        }
        let housed = total_beavers.saturating_sub(remaining_beavers) as usize;
        let mut housed_seen = 0usize;
        for b in &mut self.beavers {
            if b.is_bot == 0 {
                b.has_home = Some(if housed_seen < housed { 1 } else { 0 });
                housed_seen += 1;
            }
        }
    }

    pub fn total_food(&self) -> i64 {
        [
            "Carrot",
            "Kohlrabi",
            "FermentedCassava",
            "Berries",
            "GrilledPotato",
            "Bread",
        ]
        .iter()
        .map(|k| self.resources.get(*k).copied().unwrap_or(0.0).floor() as i64)
        .sum()
    }

    pub fn population_counts(&self) -> PopulationCounts {
        let mut adults = 0;
        let mut children = 0;
        let mut bots = 0;
        for b in &self.beavers {
            if b.is_bot == 1 {
                bots += 1;
            } else if b.life_progress.unwrap_or(0.5) < 0.2 {
                children += 1;
            } else {
                adults += 1;
            }
        }
        PopulationCounts {
            adults,
            children,
            bots,
        }
    }

    pub fn time_info(&self) -> TimeInfo {
        TimeInfo {
            day_number: self.day_number,
            day_progress: self.day_progress,
            partial_day_number: self.day_number as f64 + self.day_progress,
        }
    }

    pub fn weather_info(&self) -> WeatherInfo {
        WeatherInfo {
            cycle: self.cycle,
            cycle_day: self.cycle_day,
            cycle_length_in_days: self.cycle_length(),
            is_hazardous: self.is_hazardous,
            temperate_weather_duration: self.temperate_weather_duration,
            hazardous_weather_duration: self.hazardous_weather_duration,
        }
    }

    pub fn resources_map(&self) -> ResourcesMap {
        let mut district_res = BTreeMap::new();
        for (k, v) in &self.resources {
            let amt = v.floor() as i64;
            district_res.insert(
                k.clone(),
                GoodStock {
                    available: amt,
                    all: amt,
                },
            );
        }
        let mut out = BTreeMap::new();
        out.insert(self.district_name.clone(), district_res);
        out
    }

    pub fn alerts(&self) -> Vec<AlertItem> {
        let mut items = Vec::new();
        for b in &self.buildings {
            if b.finished == 1 && b.paused == 0 {
                let desired = b.desired_workers.unwrap_or(0);
                let assigned = b.assigned_workers.unwrap_or(0);
                if desired > assigned {
                    items.push(AlertItem {
                        alert_type: "unstaffed".to_string(),
                        id: b.id,
                        name: b.name.clone(),
                        workers: Some(format!("{assigned}/{desired}")),
                    });
                }
            }
        }
        items
    }

    pub fn summary(&self) -> Summary {
        let pop = self.population_counts();
        let total_beds: u32 = self
            .buildings
            .iter()
            .filter(|b| b.finished == 1)
            .filter_map(|b| b.max_dwellers)
            .sum();
        let occupied_beds: u32 = self
            .buildings
            .iter()
            .filter(|b| b.finished == 1)
            .filter_map(|b| b.dwellers)
            .sum();
        let homeless = pop.total_beavers().saturating_sub(occupied_beds);

        let assigned: u32 = self
            .buildings
            .iter()
            .filter(|b| b.finished == 1 && b.paused == 0)
            .filter_map(|b| b.assigned_workers)
            .sum();
        let desired: u32 = self
            .buildings
            .iter()
            .filter(|b| b.finished == 1 && b.paused == 0)
            .filter_map(|b| b.desired_workers)
            .sum();
        let vacancies = desired.saturating_sub(assigned);
        let unemployed = pop.adults.saturating_sub(assigned);

        let critical = self
            .beavers
            .iter()
            .filter(|b| b.any_critical == Some(1))
            .count() as u32;
        let miserable = self
            .beavers
            .iter()
            .filter(|b| b.wellbeing.unwrap_or(0.0) < 0.0)
            .count() as u32;
        let avg_wb = if self.beavers.is_empty() {
            0.0
        } else {
            self.beavers
                .iter()
                .map(|b| b.wellbeing.unwrap_or(0.0))
                .sum::<f64>()
                / (self.beavers.len() as f64)
        };

        let dc = self
            .buildings
            .iter()
            .find(|b| b.name.starts_with("DistrictCenter"))
            .map(|b| DistrictCenterLocation {
                x: b.x,
                y: b.y,
                z: b.z,
                entrance_x: b.x + 1,
                entrance_y: b.y - 1,
                orientation: b.orientation,
            });

        let mut res_flat = BTreeMap::new();
        for (k, v) in &self.resources {
            res_flat.insert(k.clone(), v.floor() as i64);
        }

        let alerts_vec = self.alerts();
        let mut alert_counts = BTreeMap::new();
        for a in &alerts_vec {
            *alert_counts.entry(a.alert_type.clone()).or_insert(0) += 1;
        }

        let mut bldg_counts = BTreeMap::new();
        for b in &self.buildings {
            *bldg_counts.entry(b.name.clone()).or_insert(0) += 1;
        }

        let growing = self.crops.iter().filter(|c| c.grown != Some(1)).count() as u32;
        let ready = self.crops.iter().filter(|c| c.grown == Some(1)).count() as u32;
        let mut crop_species: Vec<String> = self.crops.iter().map(|c| c.name.clone()).collect();
        crop_species.sort();
        crop_species.dedup();

        let marked_grown = self
            .trees
            .iter()
            .filter(|t| t.alive == 1 && t.marked == Some(1) && t.grown == Some(1))
            .count() as u32;
        let marked_seedling = self
            .trees
            .iter()
            .filter(|t| t.alive == 1 && t.marked == Some(1) && t.grown != Some(1))
            .count() as u32;
        let unmarked_grown = self
            .trees
            .iter()
            .filter(|t| t.alive == 1 && t.marked != Some(1) && t.grown == Some(1))
            .count() as u32;

        Summary {
            settlement: self.settlement.clone(),
            faction: self.faction.clone(),
            science: self.science,
            wellbeing: Some(
                json!({ "average": avg_wb, "critical": critical, "miserable": miserable }),
            ),
            alerts: alert_counts,
            buildings: bldg_counts,
            crops: Some(SummaryCrops {
                growing,
                ready,
                species: crop_species,
            }),
            trees: Some(SummaryTrees {
                marked_grown,
                marked_seedling,
                unmarked_grown,
            }),
            districts: vec![SummaryDistrict {
                name: self.district_name.clone(),
                population: pop,
                resources: res_flat,
                housing: Some(DistrictHousing {
                    occupied_beds,
                    total_beds,
                    homeless,
                }),
                employment: Some(DistrictEmployment {
                    assigned,
                    vacancies,
                    unemployed,
                }),
                wellbeing: Some(DistrictWellbeing {
                    average: avg_wb,
                    critical,
                    miserable,
                }),
                dc,
            }],
            food_clusters: self.food_clusters(),
            tree_clusters: self.tree_clusters(),
            time: SummaryTime {
                day_number: self.day_number,
                day_progress: self.day_progress,
                partial_day_number: self.day_number as f64 + self.day_progress,
                speed: Some(self.speed),
            },
            weather: self.weather_info(),
        }
    }

    pub fn tree_clusters(&self) -> Vec<Cluster> {
        let total = self.trees.iter().filter(|t| t.alive == 1).count() as u32;
        let grown = self
            .trees
            .iter()
            .filter(|t| t.alive == 1 && t.grown == Some(1))
            .count() as u32;
        if total == 0 {
            return Vec::new();
        }
        let mut species = BTreeMap::new();
        for t in &self.trees {
            if t.alive == 1 {
                *species.entry(t.name.clone()).or_insert(0) += 1;
            }
        }
        vec![Cluster {
            x: 26,
            y: 14,
            z: 2,
            total,
            grown,
            species,
        }]
    }

    pub fn food_clusters(&self) -> Vec<Cluster> {
        let total = self.gatherables.iter().filter(|g| g.alive == 1).count() as u32;
        if total == 0 {
            return Vec::new();
        }
        let mut species = BTreeMap::new();
        species.insert("BlueberryBush".to_string(), total);
        vec![Cluster {
            x: 10,
            y: 23,
            z: 2,
            total,
            grown: total,
            species,
        }]
    }

    pub fn tiles(&self, q: &TilesQuery) -> TilesResponse {
        let (Some(x1), Some(y1), Some(x2), Some(y2)) = (q.x1, q.y1, q.x2, q.y2) else {
            return TilesResponse {
                region: None,
                map_size: self.map_size.clone(),
                tiles: None,
            };
        };
        let min_x = x1.min(x2).max(0);
        let max_x = x1.max(x2).min(self.map_size.x as i32 - 1);
        let min_y = y1.min(y2).max(0);
        let max_y = y1.max(y2).min(self.map_size.y as i32 - 1);
        let irrigated = self.reservoir_water_depth > 0.05
            && self.river_contamination < IRRIGATION_POLLUTION_CUTOFF;
        let mut out = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // River runs along x in 9..=11
                let in_river = (9..=11).contains(&x);
                let terrain = if in_river { 1 } else { 2 };
                let water = if in_river {
                    self.reservoir_water_depth
                } else {
                    0.0
                };
                let badwater = if in_river {
                    self.river_contamination
                } else {
                    0.0
                };
                let contaminated = if in_river && badwater >= IRRIGATION_POLLUTION_CUTOFF {
                    1
                } else {
                    0
                };
                let moist = if (7..=14).contains(&x) && irrigated {
                    1
                } else {
                    0
                };
                let occupants: Vec<TileOccupant> = self
                    .buildings
                    .iter()
                    .filter(|b| b.x == x && b.y == y)
                    .map(|b| TileOccupant {
                        name: b.name.clone(),
                        z: b.z,
                    })
                    .collect();
                out.push(TileInfo {
                    x,
                    y,
                    terrain,
                    water,
                    badwater,
                    entrance: 0,
                    seedling: 0,
                    dead: 0,
                    contaminated,
                    moist,
                    occupants,
                });
            }
        }
        TilesResponse {
            region: Some(TilesRegion {
                x1: min_x,
                y1: min_y,
                x2: max_x,
                y2: max_y,
            }),
            map_size: self.map_size.clone(),
            tiles: Some(out),
        }
    }

    pub fn find_placement(&self, req: &FindPlacementRequest) -> FindPlacementResponse {
        let cx = req.x.unwrap_or(16);
        let cy = req.y.unwrap_or(16);
        let radius = req.radius.unwrap_or(8).clamp(1, 16);
        let (x1, y1, x2, y2) = if req.x1 != 0 || req.x2 != 0 || req.y1 != 0 || req.y2 != 0 {
            (
                req.x1.min(req.x2),
                req.y1.min(req.y2),
                req.x1.max(req.x2),
                req.y1.max(req.y2),
            )
        } else {
            (cx - radius, cy - radius, cx + radius, cy + radius)
        };

        let mut candidates = Vec::new();
        let needs_shore = req.prefab.contains("WaterPump");
        let needs_river = req.prefab.contains("Floodgate")
            || req.prefab.starts_with("Dam.")
            || req.prefab.contains("WaterWheel");

        for y in y1.max(1)..=y2.min(self.map_size.y as i32 - 2) {
            for x in x1.max(1)..=x2.min(self.map_size.x as i32 - 2) {
                let in_river = (9..=11).contains(&x);
                if needs_river {
                    if !in_river {
                        continue;
                    }
                } else if in_river {
                    continue;
                }
                if needs_shore && x != 12 {
                    continue;
                }
                if self.buildings.iter().any(|b| b.x == x && b.y == y) {
                    continue;
                }
                let dist = (x - 16).abs() + (y - 16).abs();
                let z = if in_river { 1 } else { 2 };
                candidates.push(PlacementCandidate {
                    x,
                    y,
                    z,
                    orientation: Orientation::South,
                    entrance_x: x,
                    entrance_y: y - 1,
                    path_access: true,
                    reachable: true,
                    near_power: false,
                    flooded: false,
                    water_depth: if in_river {
                        self.reservoir_water_depth
                    } else {
                        0.0
                    },
                    distance: dist,
                });
            }
        }
        candidates.sort_by_key(|c| c.distance);
        candidates.truncate(10);
        FindPlacementResponse {
            prefab: req.prefab.clone(),
            candidates,
        }
    }

    pub fn place_building(&mut self, req: &PlaceBuildingRequest) -> Result<Value, String> {
        if req.x < 0
            || req.y < 0
            || req.x >= self.map_size.x as i32
            || req.y >= self.map_size.y as i32
        {
            return Err("invalid_coords: out of map bounds".to_string());
        }
        if self
            .buildings
            .iter()
            .any(|b| b.x == req.x && b.y == req.y && b.z == req.z)
        {
            return Err(format!(
                "blocked: tile ({},{},{}) is already occupied",
                req.x, req.y, req.z
            ));
        }

        if let Some(prefab) = self.prefabs.iter().find(|p| p.name == req.prefab).cloned() {
            if !prefab.unlocked {
                return Err(format!("locked: prefab '{}' is not unlocked", req.prefab));
            }
            for cost in &prefab.cost {
                let have = self.resources.get(&cost.good).copied().unwrap_or(0.0);
                if have < cost.amount as f64 {
                    return Err(format!(
                        "insufficient_resources: need {} {}, have {}",
                        cost.amount,
                        cost.good,
                        have.floor() as i64
                    ));
                }
            }
            for cost in &prefab.cost {
                *self.resources.entry(cost.good.clone()).or_insert(0.0) -= cost.amount as f64;
            }
        }

        let instant = req.prefab.contains("Path") || req.prefab.contains("Flag");
        let id = self.alloc_id();
        let spec = lookup_spec(&req.prefab);
        let sb = ScenarioBuilding {
            name: req.prefab.clone(),
            x: req.x,
            y: req.y,
            z: req.z,
            orientation: req.orientation,
            finished: instant,
            paused: false,
            workers: None,
            floodgate_height: spec.and_then(|s| s.max_floodgate_height).map(|_| 0.5),
            max_height: spec.and_then(|s| s.max_floodgate_height),
            recipe: None,
            allowed_good: if req.prefab.contains("Tank") {
                Some("Water".to_string())
            } else {
                None
            },
            storage_mode: if req.prefab.contains("Tank") {
                Some("accept".to_string())
            } else {
                None
            },
        };
        let b = make_building(id, &sb);
        self.buildings.push(b);
        self.rebalance_workers_and_housing();
        Ok(json!({
            "id": id,
            "prefab": req.prefab,
            "x": req.x,
            "y": req.y,
            "z": req.z,
            "orientation": req.orientation,
            "placed": true
        }))
    }
}

fn priority_rank(p: Priority) -> u8 {
    match p {
        Priority::VeryLow => 0,
        Priority::Low => 1,
        Priority::Normal => 2,
        Priority::High => 3,
        Priority::VeryHigh => 4,
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn make_event(event: &str, day: u32, timestamp: i64, data: Option<Value>) -> ServerFrame {
    ServerFrame::Event(GameEventPayload {
        event: event.to_string(),
        day,
        timestamp,
        data,
    })
}

fn make_beaver(id: i64, name: String, district: &str, is_bot: bool, life_progress: f64) -> Beaver {
    Beaver {
        id,
        name,
        district: district.to_string(),
        x: 16,
        y: 16,
        z: 2,
        is_bot: if is_bot { 1 } else { 0 },
        contaminated: Some(0),
        deterioration: if is_bot { Some(0.1) } else { Some(0.0) },
        has_home: Some(1),
        life_progress: Some(life_progress),
        any_critical: Some(0),
        workplace: None,
        wellbeing: Some(8.0),
        overburdened: Some(0),
        carry_amount: Some(0),
        carrying: Some(String::new()),
        lifting_capacity: Some(14),
        needs: vec![
            BeaverNeed {
                id: "Thirst".to_string(),
                group: "BasicNeeds".to_string(),
                points: 0.9,
                wellbeing: 0,
                favorable: 1,
                critical: 0,
            },
            BeaverNeed {
                id: "Hunger".to_string(),
                group: "BasicNeeds".to_string(),
                points: 0.9,
                wellbeing: 0,
                favorable: 1,
                critical: 0,
            },
        ],
    }
}

fn make_building(id: i64, sb: &ScenarioBuilding) -> Building {
    let spec = lookup_spec(&sb.name);
    let max_workers = spec.map(|s| s.max_workers).unwrap_or_else(|| {
        if sb.name.contains("DistrictCenter") {
            4
        } else if sb.name.contains("Farmhouse") || sb.name.contains("FarmHouse") {
            2
        } else if sb.name.contains("WaterPump")
            || sb.name.contains("LumberjackFlag")
            || sb.name.contains("GathererFlag")
            || sb.name.contains("Forester")
            || sb.name.contains("Inventor")
            || sb.name.contains("LumberMill")
            || sb.name.contains("Fermenter")
        {
            1
        } else {
            0
        }
    });
    let desired = sb.workers.unwrap_or(max_workers).min(max_workers);
    let max_dwellers = spec.map(|s| s.max_dwellers).unwrap_or_else(|| {
        if sb.name.contains("LargeBarrack") {
            16
        } else if sb.name.contains("Barrack") {
            10
        } else if sb.name.contains("TripleLodge") {
            9
        } else if sb.name.contains("DoubleLodge") {
            6
        } else if sb.name.contains("Lodge") {
            3
        } else {
            0
        }
    });
    let capacity = spec.and_then(|s| s.storage_capacity).or_else(|| {
        if sb.name.contains("LargeTank") {
            Some(1200)
        } else if sb.name.contains("MediumTank") {
            Some(300)
        } else if sb.name.contains("SmallTank") {
            Some(30)
        } else {
            None
        }
    });
    let is_floodgate = sb.name.contains("Floodgate");
    let max_fg = sb
        .max_height
        .or_else(|| spec.and_then(|s| s.max_floodgate_height))
        .or(if is_floodgate { Some(1.0) } else { None });
    let nom_out = spec.map(|s| s.power_output).unwrap_or(0.0);
    let nom_in = spec.map(|s| s.power_input).unwrap_or(0.0);

    Building {
        id,
        name: sb.name.clone(),
        x: sb.x,
        y: sb.y,
        z: sb.z,
        orientation: sb.orientation,
        finished: if sb.finished { 1 } else { 0 },
        paused: if sb.paused { 1 } else { 0 },
        pausable: Some(1),
        construction_priority: Some(Priority::Normal),
        workplace_priority: Some(Priority::Normal),
        max_workers: Some(max_workers),
        desired_workers: Some(desired),
        assigned_workers: Some(0),
        reachable: Some(1),
        powered: Some(1),
        is_generator: Some(if nom_out > 0.0 { 1 } else { 0 }),
        is_consumer: Some(if nom_in > 0.0 { 1 } else { 0 }),
        nominal_power_input: Some(nom_in),
        nominal_power_output: Some(nom_out),
        power_demand: Some(0.0),
        power_supply: Some(0.0),
        build_progress: Some(if sb.finished { 1.0 } else { 0.0 }),
        material_progress: Some(1.0),
        has_materials: Some(1),
        stock: Some(0),
        capacity,
        storage_mode: sb.storage_mode.clone(),
        allowed_good: sb.allowed_good.clone(),
        dwellers: Some(0),
        max_dwellers: Some(max_dwellers),
        floodgate: Some(if is_floodgate { 1 } else { 0 }),
        height: sb
            .floodgate_height
            .or(if is_floodgate { Some(0.5) } else { None }),
        max_height: max_fg,
        is_clutch: Some(if sb.name.contains("Clutch") { 1 } else { 0 }),
        clutch_engaged: Some(0),
        current_recipe: sb.recipe.clone(),
        production_progress: Some(0.0),
        ready_to_produce: Some(1),
        effect_radius: spec.and_then(|s| s.effect_radius),
        is_wonder: Some(if spec.is_some_and(|s| s.is_wonder) {
            1
        } else {
            0
        }),
        needs_nutrients: Some(if sb.name.starts_with("BreedingPod") {
            1
        } else {
            0
        }),
        alerts: Some(String::new()),
        inventory: BTreeMap::new(),
        recipes: Vec::new(),
        automation: Some(BuildingAutomation {
            automation_type: Some(if sb.name.contains("DepthSensor") {
                "DepthSensor".to_string()
            } else if sb.name.contains("Relay") {
                "Relay".to_string()
            } else if sb.name.contains("Lever") {
                "Lever".to_string()
            } else {
                "Automatable".to_string()
            }),
            has_automator: Some(1),
            automator_name: Some(sb.name.clone()),
            is_automated: Some(0),
            is_transmitter: Some(if sb.name.contains("Sensor") || sb.name.contains("Lever") {
                1
            } else {
                0
            }),
            state: Some("Off".to_string()),
            input_name: Some(String::new()),
            inputs: Vec::new(),
            outputs: Vec::new(),
            config: BTreeMap::new(),
        }),
    }
}
