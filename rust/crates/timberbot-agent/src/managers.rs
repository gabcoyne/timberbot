use timberbot_protocol::http::{Building, Orientation, Priority, Summary, TilesResponse};

/// Snapshot observed while the game is paused.
#[derive(Debug, Clone)]
pub struct Observation {
    pub summary: Summary,
    pub buildings: Vec<Building>,
    pub tiles: TilesResponse,
    pub work_end_hours: u8,
}

impl Observation {
    pub fn is_iron_teeth(&self) -> bool {
        self.summary.faction.eq_ignore_ascii_case("IronTeeth")
    }

    pub fn faction_suffix(&self) -> &'static str {
        if self.is_iron_teeth() {
            "IronTeeth"
        } else {
            "Folktails"
        }
    }

    pub fn pump_prefab(&self) -> &'static str {
        if self.is_iron_teeth() {
            "DeepWaterPump.IronTeeth"
        } else {
            "WaterPump.Folktails"
        }
    }

    pub fn farm_prefab(&self) -> &'static str {
        if self.is_iron_teeth() {
            "FarmHouse.IronTeeth"
        } else {
            "EfficientFarmHouse.Folktails"
        }
    }

    pub fn housing_prefab(&self) -> &'static str {
        if self.is_iron_teeth() {
            "Barrack.IronTeeth"
        } else {
            "Lodge.Folktails"
        }
    }

    pub fn primary_crop(&self) -> &'static str {
        if self.is_iron_teeth() {
            "Kohlrabi"
        } else {
            "Carrot"
        }
    }

    pub fn total_beavers(&self) -> u32 {
        self.summary
            .districts
            .iter()
            .map(|d| d.population.total_beavers())
            .sum()
    }

    pub fn resource(&self, name: &str) -> i64 {
        self.summary
            .districts
            .iter()
            .filter_map(|d| d.resources.get(name))
            .sum()
    }

    pub fn total_food(&self) -> i64 {
        self.resource("Carrot")
            + self.resource("Kohlrabi")
            + self.resource("FermentedCassava")
            + self.resource("Berries")
            + self.resource("GrilledPotato")
            + self.resource("Bread")
    }

    pub fn homeless(&self) -> u32 {
        self.summary
            .districts
            .iter()
            .filter_map(|d| d.housing.as_ref().map(|h| h.homeless))
            .sum()
    }

    pub fn days_until_hazard(&self) -> i32 {
        let w = &self.summary.weather;
        if w.is_hazardous {
            0
        } else {
            w.temperate_weather_duration as i32 - w.cycle_day as i32 + 1
        }
    }

    pub fn max_river_contamination(&self) -> f64 {
        self.tiles
            .tiles
            .as_ref()
            .map(|ts| ts.iter().map(|t| t.badwater).fold(0.0_f64, f64::max))
            .unwrap_or(0.0)
    }

    pub fn max_river_depth(&self) -> f64 {
        self.tiles
            .tiles
            .as_ref()
            .map(|ts| ts.iter().map(|t| t.water).fold(0.0_f64, f64::max))
            .unwrap_or(0.0)
    }

    /// Estimate upcoming hazardous days so storage & food buffers scale with difficulty.
    pub fn planned_hazard_days(&self) -> i64 {
        (self.summary.weather.hazardous_weather_duration as i64).clamp(4, 30)
    }
}

/// Deterministic action planned by one of the four colony managers.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    SetFloodgate {
        id: i64,
        height: f64,
        reason: String,
    },
    PauseBuilding {
        id: i64,
        paused: bool,
        reason: String,
    },
    SetPriority {
        id: i64,
        priority: Priority,
        reason: String,
    },
    SetWorkers {
        id: i64,
        count: u32,
        reason: String,
    },
    SetWorkHours {
        end_hours: u8,
        reason: String,
    },
    FindAndPlace {
        prefab: String,
        reason: String,
    },
    PlantCrops {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        z: i32,
        crop: String,
        reason: String,
    },
    MarkTrees {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        z: i32,
        reason: String,
    },
}

/// Run the four deterministic managers (water, food, logs, housing/population) in survival-priority order,
/// tracking a shared log budget so one decision cycle never overspends colony logs.
pub fn plan_actions(obs: &Observation) -> Vec<Action> {
    let mut actions = Vec::new();
    let mut log_budget = obs.resource("Log");
    water_manager(obs, &mut log_budget, &mut actions);
    food_manager(obs, &mut log_budget, &mut actions);
    logs_manager(obs, &mut actions);
    housing_manager(obs, &mut log_budget, &mut actions);
    actions
}

/// Deterministic water manager (Folktails & Iron Teeth, Easy/Normal/Hard):
/// - Raises floodgates before/during droughts and badtides (verified direction: raising blocks flow).
/// - Pauses water pumps if river contamination >= 0.30, otherwise keeps them unpaused & VeryHigh priority.
/// - Scales SmallTank storage and WaterPump / DeepWaterPump count to cover `planned_hazard_days`.
pub fn water_manager(obs: &Observation, log_budget: &mut i64, out: &mut Vec<Action>) {
    let pop = obs.total_beavers().max(1) as i64;
    let water = obs.resource("Water");
    let hazard_days = obs.planned_hazard_days();
    let hazard_soon = obs.summary.weather.is_hazardous || obs.days_until_hazard() <= 3;
    let contaminated = obs.max_river_contamination() >= 0.3;

    // 1. Floodgate height control (verified: height 0 = open, raising blocks/retains flow)
    for b in &obs.buildings {
        if b.finished == 1 && b.floodgate == Some(1) {
            let max_h = b.max_height.unwrap_or(1.0);
            let cur_h = b.height.unwrap_or(0.0);
            let target_h = if hazard_soon || contaminated {
                (max_h * 0.75).max(1.0).min(max_h)
            } else {
                0.5_f64.min(max_h)
            };
            if (cur_h - target_h).abs() >= 0.2 {
                out.push(Action::SetFloodgate {
                    id: b.id,
                    height: target_h,
                    reason: if contaminated {
                        "raise floodgate to block contaminated river inflow".to_string()
                    } else if hazard_soon {
                        "raise floodgate to retain reservoir water for hazard".to_string()
                    } else {
                        "restore temperate floodgate height".to_string()
                    },
                });
            }
        }
    }

    // 2. WaterPump / DeepWaterPump staffing & badwater pause
    let pumps: Vec<&Building> = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("WaterPump"))
        .collect();

    for p in &pumps {
        if contaminated {
            if p.paused == 0 {
                out.push(Action::PauseBuilding {
                    id: p.id,
                    paused: true,
                    reason: "pause WaterPump while river is contaminated".to_string(),
                });
            }
        } else {
            if p.paused == 1 {
                out.push(Action::PauseBuilding {
                    id: p.id,
                    paused: false,
                    reason: "unpause WaterPump in clean water".to_string(),
                });
            }
            if p.workplace_priority != Some(Priority::VeryHigh) {
                out.push(Action::SetPriority {
                    id: p.id,
                    priority: Priority::VeryHigh,
                    reason: "keep WaterPump at VeryHigh priority".to_string(),
                });
            }
            let max_w = p.max_workers.unwrap_or(1);
            if p.desired_workers.unwrap_or(0) < max_w {
                out.push(Action::SetWorkers {
                    id: p.id,
                    count: max_w,
                    reason: "staff WaterPump fully".to_string(),
                });
            }
        }
    }

    // 3. Scale storage tanks to cover verified 2.12 water/beaver/day across full hazard duration
    let desired_water = pop * ((hazard_days * 3).clamp(12, 72));
    let tank_cap: i64 = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("Tank"))
        .map(|b| b.capacity.unwrap_or(30))
        .sum();
    let total_water_cap = 300 + tank_cap;
    let unfinished_tanks = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("Tank") && b.finished == 0)
        .count();

    let need_more_cap = total_water_cap < desired_water;
    if (need_more_cap || (water < pop * 12 && tank_cap < 90))
        && unfinished_tanks == 0
        && *log_budget >= 15
    {
        *log_budget -= 15;
        out.push(Action::FindAndPlace {
            prefab: format!("SmallTank.{}", obs.faction_suffix()),
            reason: format!(
                "add SmallTank (cap {total_water_cap} < target {desired_water} for {hazard_days}d hazard)"
            ),
        });
    }

    // 4. Scale pumps so temperate window can refill entire tank bank before next hazard
    let target_pumps = if hazard_days >= 10 || pop >= 14 { 3 } else { 2 };
    let unfinished_pumps = pumps.iter().filter(|b| b.finished == 0).count();
    if pumps.len() < target_pumps
        && unfinished_pumps == 0
        && (water < desired_water || pumps.is_empty())
        && *log_budget >= 12
    {
        *log_budget -= 12;
        out.push(Action::FindAndPlace {
            prefab: obs.pump_prefab().to_string(),
            reason: "add WaterPump to refill reserves within temperate window".to_string(),
        });
    }

    // 5. Work hours: 18h baseline, 20h during hazard or low reserves (never 24h)
    let target_hours = if (hazard_soon || water < desired_water / 2 || obs.total_food() < pop * 12)
        && obs.max_river_depth() > 0.05
    {
        20
    } else {
        18
    };
    if obs.work_end_hours != target_hours {
        out.push(Action::SetWorkHours {
            end_hours: target_hours,
            reason: "adjust work hours for hazard preparation".to_string(),
        });
    }
}

/// Deterministic food manager (Folktails & Iron Teeth):
/// - Keeps FarmHouse / EfficientFarmHouse / GathererFlag unpaused and prioritized.
/// - Plants faction-appropriate staple crop (`Carrot` for Folktails, `Kohlrabi` for Iron Teeth).
/// - Scales planted tiles to cover verified 2.67 food/beaver/day + drought reserve buffer.
pub fn food_manager(obs: &Observation, log_budget: &mut i64, out: &mut Vec<Action>) {
    let pop = obs.total_beavers().max(1) as i64;
    let food = obs.total_food();
    let hazard_days = obs.planned_hazard_days();
    let desired_food = pop * ((hazard_days * 3).clamp(14, 75));
    let food_critical = food < pop * 10 || food < desired_food / 2;

    let mut farmhouses = 0usize;
    let mut unfinished_farms = 0usize;
    let mut gatherers = 0usize;

    for b in &obs.buildings {
        if b.name.contains("Farmhouse") || b.name.contains("FarmHouse") {
            farmhouses += 1;
            if b.finished == 0 {
                unfinished_farms += 1;
            }
            if b.paused == 1 {
                out.push(Action::PauseBuilding {
                    id: b.id,
                    paused: false,
                    reason: "unpause FarmHouse".to_string(),
                });
            }
            let target_prio = if food_critical {
                Priority::VeryHigh
            } else {
                Priority::High
            };
            if b.workplace_priority != Some(target_prio) {
                out.push(Action::SetPriority {
                    id: b.id,
                    priority: target_prio,
                    reason: "prioritize FarmHouse food production".to_string(),
                });
            }
            let max_w = b.max_workers.unwrap_or(2);
            if b.desired_workers.unwrap_or(0) < max_w {
                out.push(Action::SetWorkers {
                    id: b.id,
                    count: max_w,
                    reason: "fully staff FarmHouse".to_string(),
                });
            }
        } else if b.name.contains("GathererFlag") {
            gatherers += 1;
            if (food_critical || obs.is_iron_teeth()) && b.paused == 1 {
                out.push(Action::PauseBuilding {
                    id: b.id,
                    paused: false,
                    reason: "unpause GathererFlag for berry supply".to_string(),
                });
            }
            let target_prio = if food_critical {
                Priority::VeryHigh
            } else {
                Priority::High
            };
            if b.workplace_priority != Some(target_prio) {
                out.push(Action::SetPriority {
                    id: b.id,
                    priority: target_prio,
                    reason: "keep GathererFlag prioritized for food/breeding berries".to_string(),
                });
            }
        }
    }

    if food_critical && gatherers == 0 && !obs.summary.food_clusters.is_empty() {
        out.push(Action::FindAndPlace {
            prefab: format!("GathererFlag.{}", obs.faction_suffix()),
            reason: "place GathererFlag to harvest berries during food crisis".to_string(),
        });
    }

    let target_farms = if hazard_days >= 10 || pop >= 16 { 2 } else { 1 };
    if farmhouses < target_farms && unfinished_farms == 0 && *log_budget >= 25 {
        *log_budget -= 25;
        out.push(Action::FindAndPlace {
            prefab: obs.farm_prefab().to_string(),
            reason: "build FarmHouse for sustainable food surplus".to_string(),
        });
    }

    // Verified crop math: Carrot 0.75/tile/d, Kohlrabi 0.67/tile/d -> ~5 tiles per beaver for surplus
    let crop_total = obs
        .summary
        .crops
        .as_ref()
        .map(|c| c.growing + c.ready)
        .unwrap_or(0);
    let desired_crops = (pop as u32 * 5).clamp(24, 72);
    if crop_total < desired_crops {
        // Plant across irrigated river-adjacent strips; duplicate tiles are ignored by the API.
        let (x1, y1, x2, y2) = if crop_total < 24 {
            (13, 15, 14, 24)
        } else if crop_total < 48 {
            (19, 18, 24, 24)
        } else {
            (13, 5, 14, 14)
        };
        out.push(Action::PlantCrops {
            x1,
            y1,
            x2,
            y2,
            z: 2,
            crop: obs.primary_crop().to_string(),
            reason: format!(
                "expand {} patch ({crop_total} < target {desired_crops})",
                obs.primary_crop()
            ),
        });
    }
}

/// Deterministic logs manager:
/// - Ensures a LumberjackFlag exists and stays staffed when logs are needed.
/// - Marks nearby tree clusters for cutting if logs drop below 45.
pub fn logs_manager(obs: &Observation, out: &mut Vec<Action>) {
    let logs = obs.resource("Log");
    let lumberjacks: Vec<&Building> = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("LumberjackFlag"))
        .collect();

    if lumberjacks.is_empty() {
        out.push(Action::FindAndPlace {
            prefab: format!("LumberjackFlag.{}", obs.faction_suffix()),
            reason: "place LumberjackFlag for wood supply".to_string(),
        });
    } else if logs < 45 {
        for lj in &lumberjacks {
            if lj.paused == 1 {
                out.push(Action::PauseBuilding {
                    id: lj.id,
                    paused: false,
                    reason: "unpause LumberjackFlag for logs".to_string(),
                });
            }
        }
        if let Some(cluster) = obs.summary.tree_clusters.first() {
            out.push(Action::MarkTrees {
                x1: cluster.x - 4,
                y1: cluster.y - 4,
                x2: cluster.x + 4,
                y2: cluster.y + 4,
                z: cluster.z,
                reason: "mark tree cluster for cutting".to_string(),
            });
        }
    }
}

/// Deterministic housing & population-control manager (faction-aware):
/// - Iron Teeth: pauses `BreedingPod.IronTeeth` whenever reserves are below hazard safety margin
///   or population >= 15 (stopping 5W+5B/5d drain & excess mouths); builds `Barrack.IronTeeth`
///   for homeless beavers when logs and reserves are comfortable.
/// - Folktails: builds `Lodge.Folktails` for homeless beavers only when water/food safely exceed
///   upcoming hazard reserves, capping unchecked growth before long Hard droughts.
pub fn housing_manager(obs: &Observation, log_budget: &mut i64, out: &mut Vec<Action>) {
    let pop = obs.total_beavers().max(1) as i64;
    let homeless = obs.homeless();
    let water = obs.resource("Water");
    let food = obs.total_food();
    let hazard_days = obs.planned_hazard_days();
    let safe_margin = pop * ((hazard_days * 2).clamp(6, 36));
    let hazard_soon = obs.summary.weather.is_hazardous || obs.days_until_hazard() <= 3;

    if obs.is_iron_teeth() {
        let should_pause_pods =
            hazard_soon || water < safe_margin || food < safe_margin || pop >= 15;
        for b in &obs.buildings {
            if b.name.starts_with("BreedingPod") && b.finished == 1 {
                let is_paused = b.paused == 1;
                if should_pause_pods && !is_paused {
                    out.push(Action::PauseBuilding {
                        id: b.id,
                        paused: true,
                        reason: "pause BreedingPod to conserve Water/Berries & cap pop".to_string(),
                    });
                } else if !should_pause_pods && is_paused {
                    out.push(Action::PauseBuilding {
                        id: b.id,
                        paused: false,
                        reason: "unpause BreedingPod with healthy reserves".to_string(),
                    });
                }
            }
        }
    }

    let unfinished_housing = obs
        .buildings
        .iter()
        .filter(|b| {
            (b.name.contains("Lodge") || b.name.contains("Barrack") || b.name.contains("Rowhouse"))
                && b.finished == 0
        })
        .count();

    let total_beds: u32 = obs
        .summary
        .districts
        .iter()
        .filter_map(|d| d.housing.as_ref().map(|h| h.total_beds))
        .sum();
    let housing_cost = if obs.is_iron_teeth() { 40 } else { 12 };
    let min_logs = if obs.is_iron_teeth() { 45 } else { 20 };
    // Avoid Folktails population runaway past sustainable farm output: only add Lodges
    // for currently homeless beavers when total_beds < 18 and reserves comfortably cover the hazard.
    let housing_cap_ok = obs.is_iron_teeth() || total_beds < 18;
    if homeless > 0
        && housing_cap_ok
        && unfinished_housing == 0
        && !obs.summary.weather.is_hazardous
        && water >= pop * 6
        && food >= pop * 6
        && *log_budget >= min_logs
    {
        *log_budget -= housing_cost;
        out.push(Action::FindAndPlace {
            prefab: obs.housing_prefab().to_string(),
            reason: format!(
                "build {} for {homeless} homeless beavers",
                obs.housing_prefab()
            ),
        });
    }
}

pub fn default_orientation() -> Orientation {
    Orientation::South
}
