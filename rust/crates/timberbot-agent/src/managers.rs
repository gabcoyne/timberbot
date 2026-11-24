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

/// Run the four deterministic managers (water, food, logs, housing) in survival-priority order.
pub fn plan_actions(obs: &Observation) -> Vec<Action> {
    let mut actions = Vec::new();
    water_manager(obs, &mut actions);
    food_manager(obs, &mut actions);
    logs_manager(obs, &mut actions);
    housing_manager(obs, &mut actions);
    actions
}

/// Deterministic water manager:
/// - Raises floodgates before/during droughts to hold reservoir depth, and raises/diverts on badtide.
/// - Pauses water pumps if river contamination spikes, otherwise keeps them unpaused & VeryHigh priority.
/// - Builds an extra SmallTank or WaterPump when water per beaver falls below safety margins.
pub fn water_manager(obs: &Observation, out: &mut Vec<Action>) {
    let pop = obs.total_beavers().max(1) as i64;
    let water = obs.resource("Water");
    let logs = obs.resource("Log");
    let hazard_soon = obs.summary.weather.is_hazardous || obs.days_until_hazard() <= 3;
    let contaminated = obs.max_river_contamination() >= 0.3;

    // 1. Floodgate height control
    for b in &obs.buildings {
        if b.finished == 1 && b.floodgate == Some(1) {
            let max_h = b.max_height.unwrap_or(1.0);
            let cur_h = b.height.unwrap_or(0.0);
            let target_h = if hazard_soon || contaminated {
                (max_h * 0.75).min(max_h)
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
                        "raise floodgate to retain reservoir water for drought".to_string()
                    } else {
                        "restore temperate floodgate height".to_string()
                    },
                });
            }
        }
    }

    // 2. WaterPump staffing / pause on contamination
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

    // 3. Expand water storage or pumping if low buffer and logs allow
    let desired_water = pop * 12;
    let tanks = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("Tank"))
        .count();
    let unfinished_tanks = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("Tank") && b.finished == 0)
        .count();

    if (water < desired_water || hazard_soon) && tanks < 3 && unfinished_tanks == 0 && logs >= 12 {
        out.push(Action::FindAndPlace {
            prefab: "SmallTank.Folktails".to_string(),
            reason: "add water storage buffer before/during drought".to_string(),
        });
    }

    let unfinished_pumps = pumps.iter().filter(|b| b.finished == 0).count();
    if pumps.len() < 2 && unfinished_pumps == 0 && water < pop * 8 && logs >= 18 {
        out.push(Action::FindAndPlace {
            prefab: "WaterPump.Folktails".to_string(),
            reason: "add second WaterPump to recover water buffer".to_string(),
        });
    }

    // 4. Extend work hours slightly during an active hazard if reserves are tight
    let target_hours = if hazard_soon && (water < pop * 10 || obs.total_food() < pop * 10) {
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

/// Deterministic food manager:
/// - Keeps Farmhouses and GathererFlags unpaused and prioritized when food is tight.
/// - Plants additional Carrots if crop count is low.
/// - Places a GathererFlag (0 log cost) if food is critical and none exist.
pub fn food_manager(obs: &Observation, out: &mut Vec<Action>) {
    let pop = obs.total_beavers().max(1) as i64;
    let food = obs.total_food();
    let food_critical = food < pop * 8;

    let mut farmhouses = 0usize;
    let mut gatherers = 0usize;

    for b in &obs.buildings {
        if b.name.contains("Farmhouse") {
            farmhouses += 1;
            if b.paused == 1 {
                out.push(Action::PauseBuilding {
                    id: b.id,
                    paused: false,
                    reason: "unpause Farmhouse".to_string(),
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
                    reason: "prioritize Farmhouse food production".to_string(),
                });
            }
            let max_w = b.max_workers.unwrap_or(2);
            if b.desired_workers.unwrap_or(0) < max_w {
                out.push(Action::SetWorkers {
                    id: b.id,
                    count: max_w,
                    reason: "fully staff Farmhouse".to_string(),
                });
            }
        } else if b.name.contains("GathererFlag") {
            gatherers += 1;
            if food_critical && b.paused == 1 {
                out.push(Action::PauseBuilding {
                    id: b.id,
                    paused: false,
                    reason: "unpause GathererFlag during food shortage".to_string(),
                });
            }
            if food_critical && b.workplace_priority != Some(Priority::VeryHigh) {
                out.push(Action::SetPriority {
                    id: b.id,
                    priority: Priority::VeryHigh,
                    reason: "boost GathererFlag priority during food crisis".to_string(),
                });
            }
        }
    }

    if food_critical && gatherers == 0 && !obs.summary.food_clusters.is_empty() {
        out.push(Action::FindAndPlace {
            prefab: "GathererFlag.Folktails".to_string(),
            reason: "place GathererFlag to harvest berries during food crisis".to_string(),
        });
    }

    if farmhouses == 0 && obs.resource("Log") >= 20 {
        out.push(Action::FindAndPlace {
            prefab: "Farmhouse.Folktails".to_string(),
            reason: "build Farmhouse for sustainable food".to_string(),
        });
    }

    let crop_total = obs
        .summary
        .crops
        .as_ref()
        .map(|c| c.growing + c.ready)
        .unwrap_or(0);
    if crop_total < 16 {
        out.push(Action::PlantCrops {
            x1: 13,
            y1: 16,
            x2: 14,
            y2: 20,
            z: 2,
            crop: "Carrot".to_string(),
            reason: "plant Carrot patch for steady food output".to_string(),
        });
    }
}

/// Deterministic logs manager:
/// - Ensures a LumberjackFlag exists and stays staffed when logs are low.
/// - Marks nearby tree clusters for cutting if logs drop below 30.
pub fn logs_manager(obs: &Observation, out: &mut Vec<Action>) {
    let logs = obs.resource("Log");
    let lumberjacks: Vec<&Building> = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("LumberjackFlag"))
        .collect();

    if lumberjacks.is_empty() {
        out.push(Action::FindAndPlace {
            prefab: "LumberjackFlag.Folktails".to_string(),
            reason: "place LumberjackFlag for wood supply".to_string(),
        });
    } else if logs < 30 {
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
                x1: cluster.x - 3,
                y1: cluster.y - 3,
                x2: cluster.x + 3,
                y2: cluster.y + 3,
                z: cluster.z,
                reason: "mark tree cluster for cutting".to_string(),
            });
        }
    }
}

/// Deterministic housing manager:
/// - Builds a Lodge when beavers are homeless and water/food/logs are safely above crisis levels.
pub fn housing_manager(obs: &Observation, out: &mut Vec<Action>) {
    let pop = obs.total_beavers().max(1) as i64;
    let homeless = obs.homeless();
    let water = obs.resource("Water");
    let food = obs.total_food();
    let logs = obs.resource("Log");
    let unfinished_lodges = obs
        .buildings
        .iter()
        .filter(|b| b.name.contains("Lodge") && b.finished == 0)
        .count();

    if homeless > 0
        && unfinished_lodges == 0
        && !obs.summary.weather.is_hazardous
        && water >= pop * 6
        && food >= pop * 6
        && logs >= 20
    {
        out.push(Action::FindAndPlace {
            prefab: "Lodge.Folktails".to_string(),
            reason: format!("build Lodge for {homeless} homeless beavers"),
        });
    }
}

pub fn default_orientation() -> Orientation {
    Orientation::South
}
