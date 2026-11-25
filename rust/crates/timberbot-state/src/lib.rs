//! Layer 1 (Ryan Perry 4-layer model): compact, deterministic typed colony state derived
//! purely from mod read endpoints.
//!
//! Guarantees:
//! - Same [`ModSnapshot`] ALWAYS yields byte-for-byte identical [`ColonyState`].
//! - Captures *what is about to happen*: water/food runway in hours, days until drought/badtide,
//!   upstream vs intake contamination lag, construction queue & builder ETAs, blocked
//!   prerequisites (power, science, materials, builders, irrigation), idle/shortage workers,
//!   and floodgate/irrigation readiness.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use timberbot_client::TimberbotClient;
use timberbot_protocol::http::{
    Beaver, Building, ListQuery, NotificationItem, Plant, PowerNetwork, PrefabInfo, Priority,
    ResourcesMap, ScienceInfo, Summary, TileInfo, TilesQuery, WeatherInfo, WorkHours,
};

/// Verified base beaver consumption rates (Timberborn 1.1 wiki: `2.12` water/d, `2.65` food/d).
pub const WATER_PER_BEAVER_DAY: f64 = 2.12;
pub const FOOD_PER_BEAVER_DAY: f64 = 2.65;
/// Verified irrigation pollution cutoff (`50%` badwater halts soil irrigation).
pub const IRRIGATION_POLLUTION_CUTOFF: f64 = 0.50;
/// Approximated hauling throughput matching `timberbot-mock` (`12 items / builder-hr`).
pub const HAUL_ITEMS_PER_BUILDER_HOUR: f64 = 12.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Faction {
    Folktails,
    IronTeeth,
}

impl Faction {
    pub fn detect(prefabs: &[PrefabInfo], buildings: &[Building]) -> Self {
        let is_it = prefabs.iter().any(|p| p.name.contains("IronTeeth"))
            || buildings.iter().any(|b| b.name.contains("IronTeeth"));
        if is_it {
            Self::IronTeeth
        } else {
            Self::Folktails
        }
    }

    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Folktails => "Folktails",
            Self::IronTeeth => "IronTeeth",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WeatherPhase {
    Temperate,
    DroughtApproaching,
    BadtideApproaching,
    Drought,
    Badtide,
}

impl WeatherPhase {
    pub const fn is_hazardous(self) -> bool {
        matches!(self, Self::Drought | Self::Badtide)
    }

    pub const fn is_badtide_like(self) -> bool {
        matches!(self, Self::Badtide | Self::BadtideApproaching)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FloodgateSummary {
    pub id: i64,
    pub x: i32,
    pub y: i32,
    pub finished: bool,
    pub height: f64,
    pub max_height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstructionEta {
    pub id: i64,
    pub name: String,
    pub priority: Priority,
    pub paused: bool,
    pub material_progress: f64,
    pub build_progress: f64,
    pub remaining_work_hours: f64,
    /// Cumulative queue ETA in working hours given current `active_builders`, or `None` if 0 builders.
    pub eta_hours: Option<f64>,
    /// Cumulative queue ETA in in-game days given current `active_builders` and `work_hours`.
    pub eta_days: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockedPrerequisite {
    PowerDeficit {
        consumer: String,
        deficit_hp: f64,
    },
    ScienceLocked {
        prefab: String,
        need_sp: u32,
        have_sp: u32,
    },
    MissingMaterials {
        prefab: String,
        good: String,
        need: i64,
        have: i64,
    },
    NoBuilders {
        queued_sites: u32,
    },
    DryOrContaminatedIrrigation {
        river_depth: f64,
        river_contamination: f64,
    },
}

/// Multi-day commitment carried across decision cycles so the agent does not thrash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Commitment {
    pub tactic: String,
    pub started_day: u32,
    pub target_day: u32,
    pub reason: String,
}

/// Raw mod read snapshot used as the sole deterministic input to [`ColonyState::from_snapshot`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModSnapshot {
    pub summary: Summary,
    pub weather: WeatherInfo,
    pub resources: ResourcesMap,
    pub science: ScienceInfo,
    pub workhours: WorkHours,
    pub buildings: Vec<Building>,
    pub beavers: Vec<Beaver>,
    pub prefabs: Vec<PrefabInfo>,
    pub power: Vec<PowerNetwork>,
    pub notifications: Vec<NotificationItem>,
    pub trees: Vec<Plant>,
    pub crops: Vec<Plant>,
    pub river_tiles: Vec<TileInfo>,
}

impl ModSnapshot {
    /// Fetch a complete point-in-time snapshot from the Timberbot HTTP mod (or mock).
    pub async fn fetch(client: &TimberbotClient) -> Result<Self> {
        let all = ListQuery {
            limit: Some(0),
            ..Default::default()
        };
        let summary = client.summary().await?;
        let weather = client.weather().await?;
        let resources = client.resources().await?;
        let science = client.science().await?;
        let workhours = client.workhours().await?;
        let buildings = client.buildings(&all).await?.items;
        let beavers = client.beavers(&all).await?.items;
        let prefabs = client.prefabs().await?;
        let power = client.power().await?;
        let notifications = client.notifications(&all).await?;
        let trees = client.trees(&all).await?.items;
        let crops = client.crops(&all).await?.items;
        // Sample river corridor tiles (x=9..=12, y=0..=24) to observe upstream vs intake wave lag
        let river_tiles = client
            .tiles(&TilesQuery {
                x1: Some(9),
                y1: Some(0),
                x2: Some(12),
                y2: Some(24),
            })
            .await
            .ok()
            .and_then(|r| r.tiles)
            .unwrap_or_default();

        Ok(Self {
            summary,
            weather,
            resources,
            science,
            workhours,
            buildings,
            beavers,
            prefabs,
            power,
            notifications,
            trees,
            crops,
            river_tiles,
        })
    }
}

/// Layer 1 compact deterministic typed state consumed by `decide()` and logged by Layer 3.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColonyState {
    pub faction: Faction,
    pub day: u32,
    pub cycle: u32,
    pub cycle_day: u32,
    pub work_hours: u32,
    pub active_builders: u32,

    // Hazard horizon & river wave state
    pub weather_phase: WeatherPhase,
    pub days_until_hazard: i32,
    pub hazard_duration_days: u32,
    pub river_depth: f64,
    pub river_contamination: f64,
    pub upstream_contamination: f64,
    pub irrigated_tiles: u32,
    pub contaminated_tiles: u32,

    // Stocks & survival runways
    pub population: u32,
    pub adults: u32,
    pub children: u32,
    pub water_stock: i64,
    pub water_capacity: i64,
    pub water_runway_hours: f64,
    pub food_stock: i64,
    pub food_runway_hours: f64,
    pub logs: i64,
    pub planks: i64,
    pub gears: i64,
    pub treated_planks: i64,
    pub metal_blocks: i64,
    pub berries: i64,
    pub science: u32,

    // Workforce & housing
    pub total_beds: u32,
    pub free_beds: i32,
    pub unemployed_adults: i32,
    pub worker_shortage: u32,
    pub idle_workers: u32,
    pub contaminated_beavers: u32,

    // Key infrastructure & unlock state
    pub pumps_active: u32,
    pub pumps_paused: u32,
    pub farms_active: u32,
    pub farms_paused: u32,
    pub foresters_finished: u32,
    pub foresters_queued: u32,
    pub forester_unlocked: bool,
    pub dams_finished: u32,
    pub dams_queued: u32,
    pub floodgate_unlocked: bool,
    pub floodgates: Vec<FloodgateSummary>,
    pub lumber_mills_finished: u32,
    pub lumber_mills_queued: u32,
    pub lumber_mills_unpowered: u32,
    pub power_generators_finished: u32,
    pub power_generators_queued: u32,
    pub inventors_finished: u32,
    pub power_supply: f64,
    pub power_demand: f64,
    pub grown_trees: u32,
    pub sapling_trees: u32,
    pub total_crops: u32,
    pub grown_crops: u32,
    pub breeding_pods_active: u32,

    // Queues, ETAs, prerequisites, commitments
    pub construction_queue: Vec<ConstructionEta>,
    pub blocked_prerequisites: Vec<BlockedPrerequisite>,
    pub active_commitment: Option<Commitment>,
}

impl ColonyState {
    /// Deterministically derive [`ColonyState`] from a [`ModSnapshot`].
    /// Input vectors may arrive in arbitrary order; this function sorts all collections by stable
    /// keys so the output is guaranteed identical for any equivalent game state.
    pub fn from_snapshot(snap: &ModSnapshot, active_commitment: Option<Commitment>) -> Self {
        let faction = if snap.summary.faction.eq_ignore_ascii_case("IronTeeth") {
            Faction::IronTeeth
        } else if snap.summary.faction.eq_ignore_ascii_case("Folktails") {
            Faction::Folktails
        } else {
            Faction::detect(&snap.prefabs, &snap.buildings)
        };
        let day = snap.summary.time.day_number;
        let cycle = snap.weather.cycle;
        let cycle_day = snap.weather.cycle_day;
        let work_hours = (snap.workhours.end_hours.round() as u32).clamp(1, 24);

        let mut buildings = snap.buildings.clone();
        buildings.sort_by_key(|b| b.id);

        // Population
        let (adults, children) = if !snap.summary.districts.is_empty() {
            let a = snap
                .summary
                .districts
                .iter()
                .map(|d| d.population.adults)
                .sum();
            let c = snap
                .summary
                .districts
                .iter()
                .map(|d| d.population.children)
                .sum();
            (a, c)
        } else {
            let a = snap
                .beavers
                .iter()
                .filter(|b| b.is_bot == 0 && b.life_progress.unwrap_or(0.3) >= 0.2)
                .count() as u32;
            let c = snap
                .beavers
                .iter()
                .filter(|b| b.is_bot == 0 && b.life_progress.unwrap_or(0.3) < 0.2)
                .count() as u32;
            (a, c)
        };
        let population = adults + children;

        // Resources summed across districts & runways (hours)
        let res = |name: &str| -> i64 {
            snap.resources
                .values()
                .filter_map(|dist| dist.get(name))
                .map(|g| g.available)
                .sum()
        };
        let water_stock = res("Water");
        let berries = res("Berries");
        let food_stock = res("Carrot")
            + res("Kohlrabi")
            + res("FermentedCassava")
            + res("GrilledPotato")
            + res("Bread")
            + berries;
        let logs = res("Log");
        let planks = res("Plank");
        let gears = res("Gear");
        let treated_planks = res("TreatedPlank");
        let metal_blocks = res("MetalBlock");
        let science = snap.science.points;

        let pop_f = (population.max(1)) as f64;
        let water_per_hour = (pop_f * WATER_PER_BEAVER_DAY) / 24.0;
        let food_per_hour = (pop_f * FOOD_PER_BEAVER_DAY) / 24.0;
        let water_runway_hours = round1((water_stock as f64) / water_per_hour);
        let food_runway_hours = round1((food_stock as f64) / food_per_hour);

        let mut water_capacity = 300_i64;
        for b in &buildings {
            if b.finished == 1 && b.name.contains("Tank") {
                water_capacity += b.capacity.unwrap_or(30);
            }
        }

        // River tiles: distinguish upstream (y <= 6) from colony intake (y == 14)
        let mut river_depth = 1.4_f64;
        let mut river_contamination = 0.0_f64;
        let mut upstream_contamination = 0.0_f64;
        let mut irrigated_tiles = 0u32;
        let mut contaminated_tiles = 0u32;

        if !snap.river_tiles.is_empty() {
            let mut intake_depths = Vec::new();
            let mut intake_contams = Vec::new();
            for t in &snap.river_tiles {
                if t.moist == 1 {
                    irrigated_tiles += 1;
                }
                if t.contaminated == 1 {
                    contaminated_tiles += 1;
                }
                if (9..=11).contains(&t.x) {
                    if t.y <= 6 {
                        upstream_contamination = upstream_contamination.max(t.badwater);
                    }
                    if t.y == 14 {
                        intake_depths.push(t.water);
                        intake_contams.push(t.badwater);
                    }
                }
            }
            if !intake_depths.is_empty() {
                river_depth = intake_depths.iter().sum::<f64>() / intake_depths.len() as f64;
            }
            if !intake_contams.is_empty() {
                river_contamination = intake_contams.iter().fold(0.0_f64, |acc, &v| acc.max(v));
            }
        }
        river_depth = round2(river_depth);
        river_contamination = round2(river_contamination);
        upstream_contamination = round2(upstream_contamination);

        // Weather phase detection
        let days_until_hazard = if snap.weather.is_hazardous {
            0
        } else {
            (snap.weather.temperate_weather_duration as i32 - cycle_day as i32 + 1).max(0)
        };
        let hazard_duration_days = snap.weather.hazardous_weather_duration;
        let badtide_notified = snap.notifications.iter().any(|n| {
            n.cycle == cycle
                && (n.subject.contains("Badtide")
                    || n.description.to_ascii_lowercase().contains("badtide"))
        });
        let weather_phase = if snap.weather.is_hazardous {
            if badtide_notified
                || upstream_contamination >= IRRIGATION_POLLUTION_CUTOFF
                || river_contamination >= IRRIGATION_POLLUTION_CUTOFF
            {
                WeatherPhase::Badtide
            } else {
                WeatherPhase::Drought
            }
        } else if days_until_hazard <= 3 {
            if badtide_notified {
                WeatherPhase::BadtideApproaching
            } else {
                WeatherPhase::DroughtApproaching
            }
        } else {
            WeatherPhase::Temperate
        };

        // Active builders, housing, workforce, and infrastructure counts
        let mut active_builders = 0u32;
        let mut total_beds = 0u32;
        let mut assigned_total = 0u32;
        let mut worker_shortage = 0u32;
        let mut pumps_active = 0u32;
        let mut pumps_paused = 0u32;
        let mut farms_active = 0u32;
        let mut farms_paused = 0u32;
        let mut foresters_finished = 0u32;
        let mut foresters_queued = 0u32;
        let mut dams_finished = 0u32;
        let mut dams_queued = 0u32;
        let mut floodgates = Vec::new();
        let mut lumber_mills_finished = 0u32;
        let mut lumber_mills_queued = 0u32;
        let mut lumber_mills_unpowered = 0u32;
        let mut power_generators_finished = 0u32;
        let mut power_generators_queued = 0u32;
        let mut inventors_finished = 0u32;
        let mut breeding_pods_active = 0u32;

        for b in &buildings {
            let fin = b.finished == 1;
            let paused = b.paused == 1;
            if fin {
                total_beds += b.max_dwellers.unwrap_or(0);
                if !paused {
                    let desired = b.desired_workers.unwrap_or(0);
                    let assigned = b.assigned_workers.unwrap_or(0);
                    assigned_total += assigned;
                    worker_shortage += desired.saturating_sub(assigned);
                    if b.name.starts_with("DistrictCenter") || b.name.starts_with("BuildersHut") {
                        active_builders += assigned;
                    }
                }
            }
            if b.name.contains("WaterPump") {
                if fin && !paused {
                    pumps_active += 1;
                } else if fin && paused {
                    pumps_paused += 1;
                }
            } else if b.name.contains("FarmHouse") || b.name.contains("Farmhouse") {
                if fin && !paused {
                    farms_active += 1;
                } else if fin && paused {
                    farms_paused += 1;
                }
            } else if b.name.starts_with("Forester") {
                if fin {
                    foresters_finished += 1;
                } else {
                    foresters_queued += 1;
                }
            } else if b.name.starts_with("Dam.") {
                if fin {
                    dams_finished += 1;
                } else {
                    dams_queued += 1;
                }
            } else if b.floodgate == Some(1) || b.name.contains("Floodgate") {
                floodgates.push(FloodgateSummary {
                    id: b.id,
                    x: b.x,
                    y: b.y,
                    finished: fin,
                    height: round2(b.height.unwrap_or(0.0)),
                    max_height: round2(b.max_height.unwrap_or(1.0)),
                });
            } else if b.name.contains("LumberMill") {
                if fin {
                    lumber_mills_finished += 1;
                    if !paused && b.powered == Some(0) {
                        lumber_mills_unpowered += 1;
                    }
                } else {
                    lumber_mills_queued += 1;
                }
            } else if b.name.contains("Inventor") && fin {
                inventors_finished += 1;
            } else if b.name.starts_with("BreedingPod") && fin && !paused {
                breeding_pods_active += 1;
            }
            if b.is_generator == Some(1)
                || b.name.contains("PowerWheel")
                || b.name.contains("WaterWheel")
                || b.name.contains("Windmill")
                || b.name.contains("Engine")
            {
                if fin {
                    power_generators_finished += 1;
                } else {
                    power_generators_queued += 1;
                }
            }
        }

        let free_beds = total_beds as i32 - population as i32;
        let contaminated_beavers = snap
            .beavers
            .iter()
            .filter(|b| b.is_bot == 0 && b.contaminated == Some(1))
            .count() as u32;
        let healthy_adults = adults.saturating_sub(contaminated_beavers);
        let unemployed_adults = healthy_adults as i32 - assigned_total as i32;
        let idle_workers = unemployed_adults.max(0) as u32;

        // Power totals
        let power_supply = round1(snap.power.iter().map(|n| n.supply).sum());
        let power_demand = round1(snap.power.iter().map(|n| n.demand).sum());

        // Trees & crops
        let grown_trees = snap
            .trees
            .iter()
            .filter(|t| t.alive == 1 && t.grown == Some(1))
            .count() as u32;
        let sapling_trees = snap
            .trees
            .iter()
            .filter(|t| t.alive == 1 && t.grown != Some(1))
            .count() as u32;
        let total_crops = snap.crops.iter().filter(|c| c.alive == 1).count() as u32;
        let grown_crops = snap
            .crops
            .iter()
            .filter(|c| c.alive == 1 && c.grown == Some(1))
            .count() as u32;

        // Unlock flags
        let is_unlocked = |prefix: &str| {
            snap.prefabs
                .iter()
                .any(|p| p.name.starts_with(prefix) && p.unlocked)
                || snap
                    .science
                    .unlockables
                    .iter()
                    .any(|u| u.name.starts_with(prefix) && u.unlocked == 1)
        };
        let forester_unlocked = is_unlocked("Forester.");
        let floodgate_unlocked = is_unlocked("Floodgate.");

        // Construction queue & cumulative builder ETAs (sorted by priority desc, id asc)
        let mut unfinished: Vec<&Building> = buildings.iter().filter(|b| b.finished == 0).collect();
        unfinished.sort_by(|a, b| {
            let pa = priority_rank(a.construction_priority.unwrap_or(Priority::Normal));
            let pb = priority_rank(b.construction_priority.unwrap_or(Priority::Normal));
            pb.cmp(&pa).then_with(|| a.id.cmp(&b.id))
        });

        let mut cum_builder_hours = 0.0_f64;
        let mut construction_queue = Vec::with_capacity(unfinished.len());
        let mut active_unpaused_sites = 0u32;
        for b in unfinished {
            let paused = b.paused == 1;
            let mat_prog = b.material_progress.unwrap_or(0.0).clamp(0.0, 1.0);
            let build_prog = b.build_progress.unwrap_or(0.0).clamp(0.0, 1.0);
            let total_items: f64 = snap
                .prefabs
                .iter()
                .find(|p| p.name == b.name)
                .map(|p| p.cost.iter().map(|c| c.amount as f64).sum::<f64>())
                .unwrap_or(15.0)
                .max(1.0);
            let haul_rem = (1.0 - mat_prog) * (total_items / HAUL_ITEMS_PER_BUILDER_HOUR);
            let build_rem = (1.0 - build_prog) * estimate_build_work_hours(&b.name);
            let rem_work = round2(haul_rem + build_rem);

            let (eta_hours, eta_days) = if paused || active_builders == 0 {
                (None, None)
            } else {
                active_unpaused_sites += 1;
                cum_builder_hours += rem_work / (active_builders as f64);
                (
                    Some(round2(cum_builder_hours)),
                    Some(round2(cum_builder_hours / (work_hours as f64))),
                )
            };
            if !paused && active_builders == 0 {
                active_unpaused_sites += 1;
            }

            construction_queue.push(ConstructionEta {
                id: b.id,
                name: b.name.clone(),
                priority: b.construction_priority.unwrap_or(Priority::Normal),
                paused,
                material_progress: round2(mat_prog),
                build_progress: round2(build_prog),
                remaining_work_hours: rem_work,
                eta_hours,
                eta_days,
            });
        }

        // Blocked prerequisites detection (deterministic order)
        let mut blocked_prerequisites = Vec::new();
        if active_builders == 0 && active_unpaused_sites > 0 {
            blocked_prerequisites.push(BlockedPrerequisite::NoBuilders {
                queued_sites: active_unpaused_sites,
            });
        }
        if power_demand > power_supply {
            for b in &buildings {
                if b.finished == 1
                    && b.paused == 0
                    && b.is_consumer == Some(1)
                    && b.powered == Some(0)
                {
                    blocked_prerequisites.push(BlockedPrerequisite::PowerDeficit {
                        consumer: b.name.clone(),
                        deficit_hp: round1(power_demand - power_supply),
                    });
                }
            }
        }
        if river_depth <= 0.05 || river_contamination >= IRRIGATION_POLLUTION_CUTOFF {
            blocked_prerequisites.push(BlockedPrerequisite::DryOrContaminatedIrrigation {
                river_depth,
                river_contamination,
            });
        }
        // Check science/material prerequisites for key strategic unlocks (Forester & Floodgate)
        let suffix = faction.suffix();
        let forester_name = format!("Forester.{suffix}");
        if foresters_finished == 0 && foresters_queued == 0 {
            if !forester_unlocked && science < 30 {
                blocked_prerequisites.push(BlockedPrerequisite::ScienceLocked {
                    prefab: forester_name.clone(),
                    need_sp: 30,
                    have_sp: science,
                });
            }
            if planks < 7 {
                blocked_prerequisites.push(BlockedPrerequisite::MissingMaterials {
                    prefab: forester_name,
                    good: "Plank".to_string(),
                    need: 7,
                    have: planks,
                });
            }
        }
        let fg_name = format!("Floodgate.{suffix}");
        if floodgates.is_empty() {
            if !floodgate_unlocked && science < 150 {
                blocked_prerequisites.push(BlockedPrerequisite::ScienceLocked {
                    prefab: fg_name.clone(),
                    need_sp: 150,
                    have_sp: science,
                });
            }
            if planks < 5 {
                blocked_prerequisites.push(BlockedPrerequisite::MissingMaterials {
                    prefab: fg_name,
                    good: "Plank".to_string(),
                    need: 5,
                    have: planks,
                });
            }
        }

        Self {
            faction,
            day,
            cycle,
            cycle_day,
            work_hours,
            active_builders,
            weather_phase,
            days_until_hazard,
            hazard_duration_days,
            river_depth,
            river_contamination,
            upstream_contamination,
            irrigated_tiles,
            contaminated_tiles,
            population,
            adults,
            children,
            water_stock,
            water_capacity,
            water_runway_hours,
            food_stock,
            food_runway_hours,
            logs,
            planks,
            gears,
            treated_planks,
            metal_blocks,
            berries,
            science,
            total_beds,
            free_beds,
            unemployed_adults,
            worker_shortage,
            idle_workers,
            contaminated_beavers,
            pumps_active,
            pumps_paused,
            farms_active,
            farms_paused,
            foresters_finished,
            foresters_queued,
            forester_unlocked,
            dams_finished,
            dams_queued,
            floodgate_unlocked,
            floodgates,
            lumber_mills_finished,
            lumber_mills_queued,
            lumber_mills_unpowered,
            power_generators_finished,
            power_generators_queued,
            inventors_finished,
            power_supply,
            power_demand,
            grown_trees,
            sapling_trees,
            total_crops,
            grown_crops,
            breeding_pods_active,
            construction_queue,
            blocked_prerequisites,
            active_commitment,
        }
    }

    /// Compact, deterministic text rendering of Layer 1 state for LLM/Grok Bot logs and prompts.
    pub fn to_prompt_summary(&self) -> String {
        format!(
            "FACTION={:?} DAY={} CYCLE={}({}) PHASE={:?} HAZARD_IN={}d(dur {}d) \
             WATER={}({:.1}h,cap={}) FOOD={}({:.1}h) LOG={} PLANK={} GEAR={} SP={} \
             POP={}({}a/{}c,beds={},free={}) BUILDERS={} SHORTAGE={} IDLE={} \
             RIVER(d={:.2},c={:.2},up_c={:.2},irrig={}) POWER({:.0}/{:.0}hp) \
             TREES({}g/{}s,forester={}+{}q) QUEUE={} BLOCKED={}",
            self.faction,
            self.day,
            self.cycle,
            self.cycle_day,
            self.weather_phase,
            self.days_until_hazard,
            self.hazard_duration_days,
            self.water_stock,
            self.water_runway_hours,
            self.water_capacity,
            self.food_stock,
            self.food_runway_hours,
            self.logs,
            self.planks,
            self.gears,
            self.science,
            self.population,
            self.adults,
            self.children,
            self.total_beds,
            self.free_beds,
            self.active_builders,
            self.worker_shortage,
            self.idle_workers,
            self.river_depth,
            self.river_contamination,
            self.upstream_contamination,
            self.irrigated_tiles,
            self.power_supply,
            self.power_demand,
            self.grown_trees,
            self.sapling_trees,
            self.foresters_finished,
            self.foresters_queued,
            self.construction_queue.len(),
            self.blocked_prerequisites.len(),
        )
    }

    /// Basic structural invariants for Layer 1 sanity checks (used by Layer 3 `MissKind::StateBug`).
    pub fn validate_invariants(&self) -> Vec<String> {
        let mut errs = Vec::new();
        if !self.water_runway_hours.is_finite() || self.water_runway_hours < 0.0 {
            errs.push("water_runway_hours must be finite and >= 0".to_string());
        }
        if !self.food_runway_hours.is_finite() || self.food_runway_hours < 0.0 {
            errs.push("food_runway_hours must be finite and >= 0".to_string());
        }
        if self.days_until_hazard < 0 {
            errs.push("days_until_hazard must be >= 0".to_string());
        }
        if self.river_contamination < 0.0 || self.river_contamination > 1.05 {
            errs.push("river_contamination out of [0, 1] range".to_string());
        }
        if self.work_hours == 0 || self.work_hours > 24 {
            errs.push("work_hours must be in 1..=24".to_string());
        }
        if self.active_builders == 0
            && self
                .construction_queue
                .iter()
                .any(|c| !c.paused && c.eta_hours.is_some())
        {
            errs.push("construction ETA must be None when active_builders == 0".to_string());
        }
        errs
    }
}

/// Approximated construction work-hours matching `timberbot-mock::data::build_work_hours_for`.
pub fn estimate_build_work_hours(prefab: &str) -> f64 {
    if prefab.contains("Path") || prefab.contains("Flag") || prefab.starts_with("DistrictCenter") {
        0.0
    } else if prefab.contains("SmallTank") || prefab.contains("MiniLodge") {
        4.0
    } else if prefab.contains("BreedingPod") {
        5.0
    } else if prefab.contains("WaterPump")
        || prefab.contains("Lodge")
        || prefab.contains("Inventor")
    {
        6.0
    } else if prefab.contains("TripleFloodgate") || prefab.contains("Engine") {
        16.0
    } else if prefab.contains("WoodWorkshop") || prefab.contains("Smelter") {
        18.0
    } else if prefab.contains("LargeTank") {
        24.0
    } else if prefab.contains("DoubleFloodgate")
        || prefab.contains("MediumTank")
        || prefab.contains("GearWorkshop")
        || prefab.contains("BuildersHut")
        || prefab.contains("WaterWheel")
        || prefab.contains("Barrack")
    {
        12.0
    } else if prefab.contains("Forester") || prefab.contains("LargePowerWheel") {
        10.0
    } else {
        8.0
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

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use timberbot_protocol::http::{
        GoodStock, Orientation, PopulationCounts, PrefabCost, SummaryDistrict, SummaryTime,
        Unlockable,
    };

    fn sample_snapshot() -> ModSnapshot {
        let mut d_res = BTreeMap::new();
        d_res.insert(
            "Water".to_string(),
            GoodStock {
                available: 106,
                all: 106,
            },
        );
        d_res.insert(
            "Carrot".to_string(),
            GoodStock {
                available: 132,
                all: 132,
            },
        );
        d_res.insert(
            "Log".to_string(),
            GoodStock {
                available: 30,
                all: 30,
            },
        );
        d_res.insert(
            "Plank".to_string(),
            GoodStock {
                available: 2,
                all: 2,
            },
        );
        let mut resources = BTreeMap::new();
        resources.insert("District 1".to_string(), d_res);

        let dc = Building {
            id: 1000,
            name: "DistrictCenter.Folktails".to_string(),
            x: 16,
            y: 16,
            z: 2,
            orientation: Orientation::South,
            finished: 1,
            paused: 0,
            pausable: Some(1),
            construction_priority: Some(Priority::Normal),
            workplace_priority: Some(Priority::Normal),
            max_workers: Some(4),
            desired_workers: Some(2),
            assigned_workers: Some(2),
            reachable: Some(1),
            powered: Some(1),
            is_generator: Some(0),
            is_consumer: Some(0),
            nominal_power_input: Some(0.0),
            nominal_power_output: Some(0.0),
            power_demand: Some(0.0),
            power_supply: Some(0.0),
            build_progress: Some(1.0),
            material_progress: Some(1.0),
            has_materials: Some(1),
            stock: Some(0),
            capacity: Some(300),
            storage_mode: None,
            allowed_good: None,
            dwellers: Some(0),
            max_dwellers: Some(0),
            floodgate: Some(0),
            height: None,
            max_height: None,
            is_clutch: Some(0),
            clutch_engaged: Some(0),
            current_recipe: None,
            production_progress: Some(0.0),
            ready_to_produce: Some(1),
            effect_radius: None,
            is_wonder: Some(0),
            needs_nutrients: Some(0),
            alerts: None,
            inventory: BTreeMap::new(),
            recipes: Vec::new(),
            automation: None,
        };

        let mut fg = dc.clone();
        fg.id = 1005;
        fg.name = "Floodgate.Folktails".to_string();
        fg.x = 10;
        fg.y = 12;
        fg.finished = 0;
        fg.construction_priority = Some(Priority::High);
        fg.desired_workers = Some(0);
        fg.assigned_workers = Some(0);
        fg.build_progress = Some(0.25);
        fg.material_progress = Some(1.0);
        fg.floodgate = Some(1);
        fg.height = Some(0.0);
        fg.max_height = Some(1.0);

        let mut mill = dc.clone();
        mill.id = 1002;
        mill.name = "LumberMill.Folktails".to_string();
        mill.x = 18;
        mill.y = 16;
        mill.finished = 1;
        mill.desired_workers = Some(1);
        mill.assigned_workers = Some(1);
        mill.powered = Some(0);
        mill.is_consumer = Some(1);
        mill.nominal_power_input = Some(50.0);

        let weather = WeatherInfo {
            cycle: 1,
            cycle_day: 11,
            cycle_length_in_days: 19,
            is_hazardous: false,
            temperate_weather_duration: 12,
            hazardous_weather_duration: 7,
        };

        ModSnapshot {
            summary: Summary {
                settlement: "TestColony".to_string(),
                faction: "Folktails".to_string(),
                science: 20,
                alerts: BTreeMap::new(),
                buildings: BTreeMap::new(),
                crops: None,
                trees: None,
                wellbeing: None,
                districts: vec![SummaryDistrict {
                    name: "District 1".to_string(),
                    population: PopulationCounts {
                        adults: 8,
                        children: 2,
                        bots: 0,
                    },
                    resources: BTreeMap::new(),
                    housing: None,
                    employment: None,
                    wellbeing: None,
                    dc: None,
                }],
                food_clusters: Vec::new(),
                tree_clusters: Vec::new(),
                time: SummaryTime {
                    day_number: 12,
                    day_progress: 0.5,
                    partial_day_number: 12.5,
                    speed: Some(1),
                },
                weather: weather.clone(),
            },
            weather,
            resources,
            science: ScienceInfo {
                points: 20,
                unlockables: vec![Unlockable {
                    name: "Forester.Folktails".to_string(),
                    cost: 30,
                    unlocked: 0,
                }],
            },
            workhours: WorkHours {
                end_hours: 16.0,
                are_working_hours: true,
            },
            buildings: vec![fg, mill, dc], // intentionally unsorted
            beavers: Vec::new(),
            prefabs: vec![PrefabInfo {
                name: "Floodgate.Folktails".to_string(),
                size_x: 1,
                size_y: 1,
                size_z: 3,
                science_cost: 150,
                unlocked: true,
                cost: vec![
                    PrefabCost {
                        good: "Log".to_string(),
                        amount: 10,
                    },
                    PrefabCost {
                        good: "Plank".to_string(),
                        amount: 5,
                    },
                ],
            }],
            power: vec![PowerNetwork {
                id: 1,
                supply: 0.0,
                demand: 50.0,
                buildings: Vec::new(),
            }],
            notifications: vec![NotificationItem {
                subject: "BadtideApproaching".to_string(),
                description: "Badtide begins in 2 days".to_string(),
                cycle: 1,
                cycle_day: 11,
                entity_id: None,
            }],
            trees: Vec::new(),
            crops: Vec::new(),
            river_tiles: vec![
                TileInfo {
                    x: 10,
                    y: 2,
                    terrain: 1,
                    water: 1.5,
                    badwater: 0.0,
                    entrance: 0,
                    seedling: 0,
                    dead: 0,
                    contaminated: 0,
                    moist: 1,
                    occupants: Vec::new(),
                },
                TileInfo {
                    x: 10,
                    y: 14,
                    terrain: 1,
                    water: 1.5,
                    badwater: 0.0,
                    entrance: 0,
                    seedling: 0,
                    dead: 0,
                    contaminated: 0,
                    moist: 1,
                    occupants: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn same_game_state_always_yields_identical_typed_state() {
        let snap_a = sample_snapshot();
        let mut snap_b = snap_a.clone();
        snap_b.buildings.reverse();

        let state_a = ColonyState::from_snapshot(&snap_a, None);
        let state_b = ColonyState::from_snapshot(&snap_b, None);

        assert_eq!(state_a, state_b);
        assert_eq!(
            serde_json::to_string(&state_a).unwrap(),
            serde_json::to_string(&state_b).unwrap()
        );
        assert!(state_a.validate_invariants().is_empty());
    }

    #[test]
    fn computes_runways_etas_hazard_phase_and_blocked_prerequisites() {
        let snap = sample_snapshot();
        let s = ColonyState::from_snapshot(&snap, None);

        assert_eq!(s.weather_phase, WeatherPhase::BadtideApproaching);
        assert_eq!(s.days_until_hazard, 2);
        // 10 beavers * 2.12 water/d = 21.2/d -> 106 water = 5.0 days = 120.0 hours
        assert_eq!(s.water_runway_hours, 120.0);
        // Floodgate has 75% of 8h = 6h left, 2 active builders -> 3.0h ETA
        assert_eq!(s.construction_queue.len(), 1);
        assert_eq!(s.construction_queue[0].eta_hours, Some(3.0));
        // Detects power deficit on LumberMill AND Forester science/plank prerequisites
        assert!(s
            .blocked_prerequisites
            .iter()
            .any(|b| matches!(b, BlockedPrerequisite::PowerDeficit { .. })));
        assert!(s
            .blocked_prerequisites
            .iter()
            .any(|b| matches!(b, BlockedPrerequisite::ScienceLocked { prefab, .. } if prefab == "Forester.Folktails")));
    }
}
