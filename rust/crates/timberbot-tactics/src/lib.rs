//! Layer 2 (Ryan Perry 4-layer model): faction-specific tactic catalogue, pure `decide()`
//! selector, multi-day commitments, lead-time & prerequisite checks, and deterministic
//! step-by-step playbook execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use timberbot_client::TimberbotClient;
use timberbot_protocol::http::{
    CuttingAreaRequest, FindPlacementRequest, ListQuery, Orientation, PlaceBuildingRequest,
    PlantCropRequest, Priority, SetPriorityRequest,
};
use timberbot_state::{
    estimate_build_work_hours, ColonyState, Commitment, Faction, ModSnapshot, WeatherPhase,
    HAUL_ITEMS_PER_BUILDER_HOUR, IRRIGATION_POLLUTION_CUTOFF,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TacticId {
    RecoverColonyCrisis,
    DivertBadtideFloodgates,
    EmergencyFoodRationing,
    SecureEarlyWater,
    BuildDamBeforeDrought,
    StagePlankChain,
    StageGearChain,
    UnlockAndPlantForester,
    ManageBreedingPods,
    AdvanceScienceAndQueue,
    ExpandHousingCapped,
    HoldAndMonitor,
}

impl TacticId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecoverColonyCrisis => "recover_colony_crisis",
            Self::DivertBadtideFloodgates => "divert_badtide_floodgates",
            Self::EmergencyFoodRationing => "emergency_food_rationing",
            Self::SecureEarlyWater => "secure_early_water",
            Self::BuildDamBeforeDrought => "build_dam_before_drought",
            Self::StagePlankChain => "stage_plank_chain",
            Self::StageGearChain => "stage_gear_chain",
            Self::UnlockAndPlantForester => "unlock_and_plant_forester",
            Self::ManageBreedingPods => "manage_breeding_pods",
            Self::AdvanceScienceAndQueue => "advance_science_and_queue",
            Self::ExpandHousingCapped => "expand_housing_capped",
            Self::HoldAndMonitor => "hold_and_monitor",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "recover_colony_crisis" => Some(Self::RecoverColonyCrisis),
            "divert_badtide_floodgates" => Some(Self::DivertBadtideFloodgates),
            "emergency_food_rationing" => Some(Self::EmergencyFoodRationing),
            "secure_early_water" => Some(Self::SecureEarlyWater),
            "build_dam_before_drought" => Some(Self::BuildDamBeforeDrought),
            "stage_plank_chain" => Some(Self::StagePlankChain),
            "stage_gear_chain" => Some(Self::StageGearChain),
            "unlock_and_plant_forester" => Some(Self::UnlockAndPlantForester),
            "manage_breeding_pods" => Some(Self::ManageBreedingPods),
            "advance_science_and_queue" => Some(Self::AdvanceScienceAndQueue),
            "expand_housing_capped" => Some(Self::ExpandHousingCapped),
            "hold_and_monitor" => Some(Self::HoldAndMonitor),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogueEntry {
    pub tactic: TacticId,
    pub faction: Faction,
    pub enabled: bool,
    pub lead_time_days: f64,
    pub feasible_before_hazard: bool,
    pub blocked_by: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Decision {
    pub tactic: TacticId,
    pub rationale: String,
    pub lead_time_days: f64,
    pub commitment: Option<Commitment>,
    pub catalogue: Vec<CatalogueEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
    Succeeded,
    Partial,
    Blocked,
    NoOp,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybookOutcome {
    pub tactic: TacticId,
    pub status: OutcomeStatus,
    pub steps_executed: Vec<String>,
    pub error: Option<String>,
}

/// Estimate days required to haul + build a single prefab given current builders & workday.
pub fn estimate_lead_time_days(state: &ColonyState, prefab: &str, total_items: f64) -> f64 {
    let builders = state.active_builders.max(1) as f64;
    let wh = state.work_hours.max(1) as f64;
    let haul_h = total_items / HAUL_ITEMS_PER_BUILDER_HOUR;
    let build_h = estimate_build_work_hours(prefab);
    round2((haul_h + build_h) / (builders * wh))
}

fn fits_before_hazard(state: &ColonyState, lead_time_days: f64) -> bool {
    if state.weather_phase.is_hazardous() {
        lead_time_days <= 0.1
    } else {
        lead_time_days <= (state.days_until_hazard as f64).max(0.0)
    }
}

/// Build the faction-specific catalogue of tactics available for the given typed state,
/// complete with lead-time estimates and prerequisite blockers.
pub fn available_catalogue(state: &ColonyState) -> Vec<CatalogueEntry> {
    let f = state.faction;
    let suffix = f.suffix();
    let mut entries = Vec::with_capacity(12);

    // 0. recover_colony_crisis
    {
        let mut blocked_by = Vec::new();
        let has_crisis = state.unreachable_buildings > 0
            || state.disconnected_power_consumers > 0
            || (state.water_runway_hours < 36.0 && state.food_runway_hours < 36.0)
            || (state.contaminated_beavers > 0
                && (state.pumps_paused > 0 || state.farms_paused > 0));
        if !has_crisis {
            blocked_by.push(
                "no multi-system crisis, unreachable site, or disconnected power".to_string(),
            );
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::RecoverColonyCrisis,
            faction: f,
            enabled: has_crisis,
            lead_time_days: 0.25,
            feasible_before_hazard: true,
            blocked_by,
            description: "Reconnect isolated sites with Paths, bridge disconnected power networks with PowerShafts, and triage dual water+food collapse".to_string(),
        });
    }

    // 1. divert_badtide_floodgates
    {
        let mut blocked_by = Vec::new();
        let has_finished_fg = state.floodgates.iter().any(|fg| fg.finished);
        let unfinished_fg = state
            .construction_queue
            .iter()
            .find(|c| c.name.contains("Floodgate"));
        let lead = if has_finished_fg {
            0.0
        } else if let Some(q) = unfinished_fg {
            q.eta_days
                .unwrap_or_else(|| estimate_lead_time_days(state, &q.name, 15.0))
        } else {
            estimate_lead_time_days(state, &format!("Floodgate.{suffix}"), 15.0)
        };
        if !has_finished_fg && unfinished_fg.is_none() {
            if !state.floodgate_unlocked && state.science < 150 {
                blocked_by.push(format!("needs 150 SP (have {})", state.science));
            }
            if state.logs < 10 {
                blocked_by.push(format!("needs 10 Log (have {})", state.logs));
            }
            if state.planks < 5 {
                blocked_by.push(format!("needs 5 Plank (have {})", state.planks));
            }
        }
        let feasible = has_finished_fg
            || unfinished_fg.is_some()
            || (blocked_by.is_empty() && fits_before_hazard(state, lead));
        let enabled = has_finished_fg || unfinished_fg.is_some() || blocked_by.is_empty();
        entries.push(CatalogueEntry {
            tactic: TacticId::DivertBadtideFloodgates,
            faction: f,
            enabled,
            lead_time_days: lead,
            feasible_before_hazard: feasible,
            blocked_by,
            description: "Rush/raise floodgates and pause contaminated pumps before/during badtide, then resume once clean".to_string(),
        });
    }

    // 2. emergency_food_rationing
    {
        let mut blocked_by = Vec::new();
        let farm_prefab = match f {
            Faction::Folktails => "EfficientFarmHouse.Folktails",
            Faction::IronTeeth => "FarmHouse.IronTeeth",
        };
        let need_new_farm = state.farms_active == 0 && state.farms_paused == 0;
        let lead = if need_new_farm {
            estimate_lead_time_days(state, farm_prefab, 25.0)
        } else {
            0.0
        };
        if need_new_farm && state.logs < 25 {
            blocked_by.push(format!("needs 25 Log for Farmhouse (have {})", state.logs));
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::EmergencyFoodRationing,
            faction: f,
            enabled: !need_new_farm || blocked_by.is_empty(),
            lead_time_days: lead,
            feasible_before_hazard: true,
            blocked_by,
            description: "Unpause/prioritize farms, extend workday, gather berries, and plant fast crops during food crunch".to_string(),
        });
    }

    // 3. secure_early_water
    {
        let mut blocked_by = Vec::new();
        let pump_prefab = match f {
            Faction::Folktails => "WaterPump.Folktails",
            Faction::IronTeeth => "DeepWaterPump.IronTeeth",
        };
        let tank_prefab = format!("SmallTank.{suffix}");
        let need_pump = state.pumps_active == 0 && state.pumps_paused == 0;
        let lead = if need_pump {
            estimate_lead_time_days(state, pump_prefab, 12.0)
        } else {
            estimate_lead_time_days(state, &tank_prefab, 15.0)
        };
        if need_pump && state.logs < 12 {
            blocked_by.push(format!("needs 12 Log for WaterPump (have {})", state.logs));
        } else if !need_pump && state.pumps_paused == 0 && state.logs < 15 {
            blocked_by.push(format!("needs 15 Log for SmallTank (have {})", state.logs));
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::SecureEarlyWater,
            faction: f,
            enabled: state.pumps_paused > 0 || blocked_by.is_empty(),
            lead_time_days: lead,
            feasible_before_hazard: fits_before_hazard(state, lead),
            blocked_by,
            description: format!("Ensure staffed {pump_prefab} and {tank_prefab} water buffer"),
        });
    }

    // 4. build_dam_before_drought
    {
        let mut blocked_by = Vec::new();
        let dam_prefab = format!("Dam.{suffix}");
        let lead = if let Some(q) = state
            .construction_queue
            .iter()
            .find(|c| c.name.starts_with("Dam."))
        {
            q.eta_days
                .unwrap_or_else(|| estimate_lead_time_days(state, &dam_prefab, 20.0))
        } else {
            estimate_lead_time_days(state, &dam_prefab, 20.0)
        };
        if state.dams_queued == 0 && state.logs < 20 {
            blocked_by.push(format!("needs 20 Log for Dam (have {})", state.logs));
        }
        let feasible = fits_before_hazard(state, lead);
        if !feasible && state.dams_queued == 0 {
            blocked_by.push(format!(
                "lead time {:.2}d exceeds {}d until hazard",
                lead, state.days_until_hazard
            ));
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::BuildDamBeforeDrought,
            faction: f,
            enabled: state.dams_finished == 0
                && (state.dams_queued > 0 || (blocked_by.is_empty() && feasible)),
            lead_time_days: lead,
            feasible_before_hazard: feasible,
            blocked_by,
            description: "Place and prioritize a river Dam before drought when lead time fits"
                .to_string(),
        });
    }

    // 5. stage_plank_chain
    {
        let mut blocked_by = Vec::new();
        let gen_prefab = match f {
            Faction::Folktails => "PowerWheel.Folktails",
            Faction::IronTeeth => "LargePowerWheel.IronTeeth",
        };
        let mill_prefab = format!("LumberMill.{suffix}");
        let need_power = state.power_generators_finished == 0
            && state.power_generators_queued == 0
            && (state.lumber_mills_finished > 0
                || state.lumber_mills_queued > 0
                || state.power_demand >= state.power_supply);
        let gen_cost = match f {
            Faction::Folktails => 10,
            Faction::IronTeeth => 25,
        };
        let lead = if need_power {
            estimate_lead_time_days(state, gen_prefab, gen_cost as f64)
                + estimate_lead_time_days(state, &mill_prefab, 15.0)
        } else {
            estimate_lead_time_days(state, &mill_prefab, 15.0)
        };
        if state.power_generators_finished == 0
            && state.power_generators_queued == 0
            && state.logs < gen_cost
        {
            blocked_by.push(format!(
                "needs {gen_cost} Log for {gen_prefab} (have {})",
                state.logs
            ));
        } else if state.lumber_mills_finished == 0
            && state.lumber_mills_queued == 0
            && state.logs < 15
        {
            blocked_by.push(format!("needs 15 Log for LumberMill (have {})", state.logs));
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::StagePlankChain,
            faction: f,
            enabled: blocked_by.is_empty(),
            lead_time_days: lead,
            feasible_before_hazard: true,
            blocked_by,
            description: format!(
                "Enforce power-before-mill precedence ({gen_prefab} -> {mill_prefab}) to supply Planks"
            ),
        });
    }

    // 6. stage_gear_chain (mid/late-game: 120 hp power + 100 SP + 15 Log + 25 Plank -> GearWorkshop)
    {
        let mut blocked_by = Vec::new();
        let gw_prefab = format!("GearWorkshop.{suffix}");
        let need_extra_power = state.power_supply < state.power_demand + 120.0
            && state.gear_workshops_finished == 0
            && state.gear_workshops_queued == 0;
        let lead = estimate_lead_time_days(state, &gw_prefab, 40.0)
            + if need_extra_power { 0.5 } else { 0.0 };
        if !state.gear_workshop_unlocked && state.science < 100 {
            blocked_by.push(format!(
                "needs 100 SP for GearWorkshop (have {})",
                state.science
            ));
        }
        if state.gear_workshops_finished == 0 && state.gear_workshops_queued == 0 {
            if state.logs < 15 {
                blocked_by.push(format!(
                    "needs 15 Log for GearWorkshop (have {})",
                    state.logs
                ));
            }
            if state.planks < 25 {
                blocked_by.push(format!(
                    "needs 25 Plank for GearWorkshop (have {})",
                    state.planks
                ));
            }
            if need_extra_power && state.logs < 35 {
                blocked_by.push(format!(
                    "needs >= 120 hp surplus power or 35 Log for WaterWheel/PowerWheel (have {:.0} hp, {} Log)",
                    (state.power_supply - state.power_demand).max(0.0),
                    state.logs
                ));
            }
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::StageGearChain,
            faction: f,
            enabled: !state.weather_phase.is_hazardous() && blocked_by.is_empty(),
            lead_time_days: round2(lead),
            feasible_before_hazard: fits_before_hazard(state, lead),
            blocked_by,
            description: format!(
                "Ensure >=120 hp connected power, unlock/build {gw_prefab} (15 Log + 25 Plank), and produce Gears"
            ),
        });
    }

    // 7. unlock_and_plant_forester
    {
        let mut blocked_by = Vec::new();
        let forester_prefab = format!("Forester.{suffix}");
        let lead = estimate_lead_time_days(state, &forester_prefab, 17.0);
        if !state.forester_unlocked && state.science < 30 {
            blocked_by.push(format!("needs 30 SP (have {})", state.science));
        }
        if state.logs < 10 {
            blocked_by.push(format!("needs 10 Log (have {})", state.logs));
        }
        if state.planks < 7 {
            blocked_by.push(format!("needs 7 Plank (have {})", state.planks));
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::UnlockAndPlantForester,
            faction: f,
            enabled: state.foresters_finished == 0
                && state.foresters_queued == 0
                && blocked_by.is_empty(),
            lead_time_days: lead,
            feasible_before_hazard: true,
            blocked_by,
            description:
                "Unlock and place a Forester to replenish Pine groves before timber runs out"
                    .to_string(),
        });
    }

    // 8. manage_breeding_pods (Iron Teeth population & berry/bed throttle)
    {
        let mut blocked_by = Vec::new();
        if f != Faction::IronTeeth {
            blocked_by.push("Iron Teeth faction only".to_string());
        }
        let has_any_pod = state.breeding_pods_active > 0
            || state.breeding_pods_paused > 0
            || state.breeding_pods_queued > 0;
        let can_build_pod = !has_any_pod && state.logs >= 20 && state.berries >= 15;
        if f == Faction::IronTeeth && !has_any_pod && !can_build_pod {
            blocked_by.push(format!(
                "needs 20 Log and 15 Berries to place BreedingPod (have {} Log, {} Berries)",
                state.logs, state.berries
            ));
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::ManageBreedingPods,
            faction: f,
            enabled: f == Faction::IronTeeth && (has_any_pod || can_build_pod),
            lead_time_days: if has_any_pod { 0.0 } else { estimate_lead_time_days(state, "BreedingPod.IronTeeth", 20.0) },
            feasible_before_hazard: true,
            blocked_by,
            description: "Iron Teeth: pause BreedingPods when beds/berries/runways are tight; resume/place when healthy".to_string(),
        });
    }

    // 9. advance_science_and_queue
    {
        let mut blocked_by = Vec::new();
        let inv_prefab = format!("Inventor.{suffix}");
        let need_inv = state.inventors_finished == 0;
        let lead = if state.active_builders == 0 && !state.construction_queue.is_empty() {
            0.0
        } else {
            estimate_lead_time_days(state, &inv_prefab, 12.0)
        };
        if state.active_builders > 0 && need_inv && state.logs < 12 {
            blocked_by.push(format!("needs 12 Log for Inventor (have {})", state.logs));
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::AdvanceScienceAndQueue,
            faction: f,
            enabled: (state.active_builders == 0 && !state.construction_queue.is_empty())
                || (need_inv && blocked_by.is_empty()),
            lead_time_days: lead,
            feasible_before_hazard: true,
            blocked_by,
            description:
                "Restore stalled builders or build/staff Inventor to unlock Forester and Floodgates"
                    .to_string(),
        });
    }

    // 8. expand_housing_capped
    {
        let mut blocked_by = Vec::new();
        let house_prefab = match f {
            Faction::Folktails => "Lodge.Folktails",
            Faction::IronTeeth => "Barrack.IronTeeth",
        };
        let house_logs = match f {
            Faction::Folktails => 12,
            Faction::IronTeeth => 40,
        };
        let lead = estimate_lead_time_days(state, house_prefab, house_logs as f64);
        if state.water_runway_hours < 72.0 {
            blocked_by.push(format!(
                "water runway {:.1}h < 72h cap",
                state.water_runway_hours
            ));
        }
        if state.food_runway_hours < 72.0 {
            blocked_by.push(format!(
                "food runway {:.1}h < 72h cap",
                state.food_runway_hours
            ));
        }
        if state.logs < house_logs {
            blocked_by.push(format!(
                "needs {house_logs} Log for {house_prefab} (have {})",
                state.logs
            ));
        }
        if state.population >= 24 {
            blocked_by.push("population cap (24) reached".to_string());
        }
        entries.push(CatalogueEntry {
            tactic: TacticId::ExpandHousingCapped,
            faction: f,
            enabled: !state.weather_phase.is_hazardous() && blocked_by.is_empty(),
            lead_time_days: lead,
            feasible_before_hazard: fits_before_hazard(state, lead),
            blocked_by,
            description: format!("Expand {house_prefab} only while water & food runways >= 72h"),
        });
    }

    // 9. hold_and_monitor (always enabled)
    entries.push(CatalogueEntry {
        tactic: TacticId::HoldAndMonitor,
        faction: f,
        enabled: true,
        lead_time_days: 0.0,
        feasible_before_hazard: true,
        blocked_by: Vec::new(),
        description: "Hold steady while queued sites finish and stocks remain healthy".to_string(),
    });

    entries
}

/// Pure Layer 2 selector: picks ONE named tactic from [`available_catalogue`] using
/// lead-time checks, prerequisite checks, and multi-day commitment continuity.
///
/// Called both at runtime and directly by Layer 3 offline evals with NO live game.
pub fn decide(state: &ColonyState) -> Decision {
    let catalogue = available_catalogue(state);
    let entry_of = |id: TacticId| {
        catalogue
            .iter()
            .find(|e| e.tactic == id)
            .cloned()
            .expect("catalogue contains all tactics")
    };
    let pick = |id: TacticId, rationale: String, commit_days: Option<u32>| {
        let e = entry_of(id);
        let commitment = commit_days.map(|days| Commitment {
            tactic: id.as_str().to_string(),
            started_day: state.day,
            target_day: state.day.saturating_add(days),
            reason: rationale.clone(),
        });
        Decision {
            tactic: id,
            rationale,
            lead_time_days: e.lead_time_days,
            commitment,
            catalogue: catalogue.clone(),
        }
    };

    // --- Priority 0: Restore stalled builders if urgent sites are queued ---
    if state.active_builders == 0 && !state.construction_queue.is_empty() {
        return pick(
            TacticId::AdvanceScienceAndQueue,
            format!(
                "0 active builders with {} queued construction sites — restore builders first",
                state.construction_queue.len()
            ),
            Some(1),
        );
    }

    // --- Priority 0.5: Multi-system crisis, unreachable site, or disconnected power network ---
    let crisis_entry = entry_of(TacticId::RecoverColonyCrisis);
    if crisis_entry.enabled {
        return pick(
            TacticId::RecoverColonyCrisis,
            format!(
                "Colony crisis/topology break (unreachable={}, disconnected_power={}, water={:.1}h, food={:.1}h, contam_beavers={}): recover connectivity and essentials",
                state.unreachable_buildings,
                state.disconnected_power_consumers,
                state.water_runway_hours,
                state.food_runway_hours,
                state.contaminated_beavers
            ),
            Some(1),
        );
    }

    // --- Priority 1: Badtide / upstream badwater wave / approaching badtide with floodgate ---
    let badtide_now = state.weather_phase == WeatherPhase::Badtide
        || state.upstream_contamination >= 0.25
        || state.river_contamination >= IRRIGATION_POLLUTION_CUTOFF;
    let has_open_finished_fg = state
        .floodgates
        .iter()
        .any(|fg| fg.finished && fg.height < fg.max_height - 0.1);
    let has_closed_finished_fg = state
        .floodgates
        .iter()
        .any(|fg| fg.finished && fg.height >= 0.8);
    let has_unfinished_fg = state
        .construction_queue
        .iter()
        .any(|c| c.name.contains("Floodgate"));

    if badtide_now
        && (has_open_finished_fg
            || (state.pumps_active > 0 && state.river_contamination >= IRRIGATION_POLLUTION_CUTOFF))
    {
        return pick(
            TacticId::DivertBadtideFloodgates,
            format!(
                "Badtide/badwater active (intake={:.2}, upstream={:.2}): raise floodgates and pause contaminated pumps",
                state.river_contamination, state.upstream_contamination
            ),
            Some(state.hazard_duration_days.max(1)),
        );
    }
    // Recover after badtide clears: lower floodgates and resume paused pumps
    if !badtide_now
        && !state.weather_phase.is_badtide_like()
        && state.river_contamination < 0.15
        && state.upstream_contamination < 0.15
        && (has_closed_finished_fg && !state.weather_phase.is_hazardous()
            || (state.pumps_paused > 0 && state.river_depth > 0.05))
    {
        return pick(
            TacticId::DivertBadtideFloodgates,
            "River is clean and temperate: lower floodgates and resume paused water pumps"
                .to_string(),
            None,
        );
    }
    // Approaching badtide (<= 3 days): rush unbuilt floodgate if queued OR buildable in time!
    if state.weather_phase == WeatherPhase::BadtideApproaching && state.days_until_hazard <= 3 {
        let fg_entry = entry_of(TacticId::DivertBadtideFloodgates);
        if has_unfinished_fg {
            return pick(
                TacticId::DivertBadtideFloodgates,
                format!(
                    "Badtide in {}d with unfinished floodgate (ETA {:.2}d): set VeryHigh priority & extend workhours to finish before arrival",
                    state.days_until_hazard, fg_entry.lead_time_days
                ),
                Some(state.days_until_hazard.max(1) as u32),
            );
        }
        if !state.floodgates.iter().any(|fg| fg.finished)
            && fg_entry.enabled
            && fg_entry.feasible_before_hazard
        {
            return pick(
                TacticId::DivertBadtideFloodgates,
                format!(
                    "Badtide in {}d and floodgate lead time {:.2}d fits: unlock/place/rush floodgate now",
                    state.days_until_hazard, fg_entry.lead_time_days
                ),
                Some(state.days_until_hazard.max(1) as u32),
            );
        }
        if has_open_finished_fg && state.days_until_hazard <= 1 {
            return pick(
                TacticId::DivertBadtideFloodgates,
                "Badtide arrives within 1d: pre-raise floodgate ahead of badwater wave".to_string(),
                Some(2),
            );
        }
    }

    // --- Priority 2: Food crunch (especially mid-drought food crunch!) ---
    let food_critical = state.food_runway_hours < 48.0
        || (state.weather_phase.is_hazardous() && state.food_runway_hours < 72.0)
        || (state.farms_paused > 0 && state.food_runway_hours < 120.0)
        || (state.farms_active == 0 && state.farms_paused == 0 && state.total_crops < 12);
    if food_critical && entry_of(TacticId::EmergencyFoodRationing).enabled {
        return pick(
            TacticId::EmergencyFoodRationing,
            format!(
                "Food crunch (runway={:.1}h, phase={:?}, farms={}a/{}p): unpause/prioritize farms & ration labor",
                state.food_runway_hours, state.weather_phase, state.farms_active, state.farms_paused
            ),
            Some(2),
        );
    }

    // --- Priority 3: Early / emergency water security ---
    let target_water_hours = if state.weather_phase.is_hazardous() {
        48.0
    } else {
        (state.hazard_duration_days as f64 * 24.0 * 0.85).clamp(48.0, 120.0)
    };
    let clean_water_available =
        state.river_depth > 0.05 && state.river_contamination < IRRIGATION_POLLUTION_CUTOFF;
    let water_short = (state.pumps_active == 0 && clean_water_available)
        || (state.water_runway_hours < target_water_hours
            && (state.water_stock + 25 >= state.water_capacity || state.pumps_active == 0))
        || (state.water_runway_hours < 48.0 && clean_water_available);
    if water_short && entry_of(TacticId::SecureEarlyWater).enabled {
        return pick(
            TacticId::SecureEarlyWater,
            format!(
                "Water runway {:.1}h below target {:.1}h (cap {}, pumps {}): expand pump/tank buffer",
                state.water_runway_hours, target_water_hours, state.water_capacity, state.pumps_active
            ),
            Some(2),
        );
    }

    // --- Priority 4: Honor active multi-day commitment if still actionable & on schedule ---
    if let Some(commit) = &state.active_commitment {
        if state.day <= commit.target_day {
            if let Some(id) = TacticId::from_name(&commit.tactic) {
                let e = entry_of(id);
                if e.enabled && id != TacticId::HoldAndMonitor {
                    return Decision {
                        tactic: id,
                        rationale: format!(
                            "Continuing multi-day commitment '{}' (day {}/{}): {}",
                            commit.tactic, state.day, commit.target_day, commit.reason
                        ),
                        lead_time_days: e.lead_time_days,
                        commitment: Some(commit.clone()),
                        catalogue,
                    };
                }
            }
        }
    }

    // --- Priority 5: Build dam before drought (strict lead-time check!) ---
    let dam_entry = entry_of(TacticId::BuildDamBeforeDrought);
    let no_water_retention =
        state.dams_finished == 0 && !state.floodgates.iter().any(|fg| fg.finished);
    if no_water_retention
        && dam_entry.enabled
        && (state.dams_queued > 0
            || (dam_entry.feasible_before_hazard && state.days_until_hazard <= 6))
    {
        return pick(
            TacticId::BuildDamBeforeDrought,
            format!(
                "No dam/floodgate yet with hazard in {}d (lead time {:.2}d fits): build/prioritize Dam",
                state.days_until_hazard, dam_entry.lead_time_days
            ),
            Some(3),
        );
    }

    // --- Priority 6: Stage plank & power chain (strict power-before-mill precedence!) ---
    let plank_entry = entry_of(TacticId::StagePlankChain);
    let need_power_for_mill =
        state.lumber_mills_unpowered > 0 || (state.power_demand > state.power_supply);
    let need_planks_for_prereqs = state.planks < 12
        && (state.foresters_finished == 0 || state.floodgates.is_empty())
        && (state.lumber_mills_finished == 0
            || state.power_generators_finished == 0
            || state.lumber_mills_unpowered > 0);
    if (need_power_for_mill || need_planks_for_prereqs) && plank_entry.enabled {
        return pick(
            TacticId::StagePlankChain,
            format!(
                "Stage power & plank chain (power {:.0}/{:.0}hp, mills={}, planks={})",
                state.power_supply, state.power_demand, state.lumber_mills_finished, state.planks
            ),
            Some(3),
        );
    }

    // --- Priority 7: Unlock and plant Forester (when timber low & prereqs satisfied!) ---
    let forester_entry = entry_of(TacticId::UnlockAndPlantForester);
    let timber_pressure = state.grown_trees < 25 || state.logs < 45;
    if timber_pressure && forester_entry.enabled {
        return pick(
            TacticId::UnlockAndPlantForester,
            format!(
                "Timber pressure (logs={}, grown_trees={}) with SP={} & planks={}: unlock and place Forester",
                state.logs, state.grown_trees, state.science, state.planks
            ),
            Some(3),
        );
    }

    // --- Priority 8: Advance science (Inventor) when Forester/Floodgate unlocks need SP ---
    let sci_entry = entry_of(TacticId::AdvanceScienceAndQueue);
    if sci_entry.enabled && state.inventors_finished == 0 && state.science < 150 {
        return pick(
            TacticId::AdvanceScienceAndQueue,
            format!(
                "Science {} SP < 150 and no Inventor: build Inventor to unlock Forester/Floodgate",
                state.science
            ),
            Some(2),
        );
    }

    // --- Priority 8.5: Iron Teeth BreedingPod throttle / resume ---
    let pod_entry = entry_of(TacticId::ManageBreedingPods);
    if state.faction == Faction::IronTeeth && pod_entry.enabled {
        let should_pause_pods = state.breeding_pods_active > 0
            && (state.free_beds <= 0
                || state.berries < 15
                || state.water_runway_hours < 60.0
                || state.food_runway_hours < 60.0
                || state.weather_phase.is_hazardous());
        let should_resume_or_build_pods = !state.weather_phase.is_hazardous()
            && state.breeding_pods_active == 0
            && state.breeding_pods_queued == 0
            && state.free_beds >= 2
            && state.berries >= 15
            && state.water_runway_hours >= 72.0
            && state.food_runway_hours >= 72.0
            && state.population < 22;
        if should_pause_pods {
            return pick(
                TacticId::ManageBreedingPods,
                format!(
                    "Iron Teeth pod throttle (active_pods={}, free_beds={}, berries={}, water={:.1}h, food={:.1}h, phase={:?})",
                    state.breeding_pods_active,
                    state.free_beds,
                    state.berries,
                    state.water_runway_hours,
                    state.food_runway_hours,
                    state.weather_phase
                ),
                Some(1),
            );
        }
        if should_resume_or_build_pods {
            return pick(
                TacticId::ManageBreedingPods,
                format!(
                    "Iron Teeth pod resume/build (paused_pods={}, free_beds={}, berries={}, runways healthy)",
                    state.breeding_pods_paused, state.free_beds, state.berries
                ),
                Some(2),
            );
        }
    }

    // --- Priority 8.75: Mid-game GearWorkshop & power expansion chain ---
    let gear_entry = entry_of(TacticId::StageGearChain);
    if state.gears < 10
        && state.gear_workshops_finished == 0
        && state.gear_workshops_queued == 0
        && gear_entry.enabled
        && gear_entry.feasible_before_hazard
    {
        return pick(
            TacticId::StageGearChain,
            format!(
                "Mid-game gear bottleneck (gears={}, planks={}, sp={}, power={:.0}/{:.0}hp): stage GearWorkshop chain",
                state.gears, state.planks, state.science, state.power_supply, state.power_demand
            ),
            Some(3),
        );
    }

    // --- Priority 9: Capped housing expansion (only with healthy water & food runways) ---
    let house_entry = entry_of(TacticId::ExpandHousingCapped);
    if state.free_beds <= 1 && house_entry.enabled {
        return pick(
            TacticId::ExpandHousingCapped,
            format!(
                "Free beds {} <= 1 with healthy runways (water {:.1}h, food {:.1}h): expand capped housing",
                state.free_beds, state.water_runway_hours, state.food_runway_hours
            ),
            Some(2),
        );
    }

    pick(
        TacticId::HoldAndMonitor,
        "No urgent deficit or unlocked build needed; monitor active queues and stocks".to_string(),
        None,
    )
}

/// Deterministic Layer 2 playbook executor: runs sequential mod writes for the chosen tactic.
pub async fn execute_tactic(
    client: &TimberbotClient,
    state: &ColonyState,
    decision: &Decision,
) -> PlaybookOutcome {
    match run_playbook(client, state, decision.tactic).await {
        Ok((status, steps)) => PlaybookOutcome {
            tactic: decision.tactic,
            status,
            steps_executed: steps,
            error: None,
        },
        Err(e) => PlaybookOutcome {
            tactic: decision.tactic,
            status: OutcomeStatus::Failed,
            steps_executed: Vec::new(),
            error: Some(e.to_string()),
        },
    }
}

async fn run_playbook(
    client: &TimberbotClient,
    state: &ColonyState,
    tactic: TacticId,
) -> Result<(OutcomeStatus, Vec<String>)> {
    let mut steps = Vec::new();
    let suffix = state.faction.suffix();
    let all = ListQuery {
        limit: Some(0),
        ..Default::default()
    };
    let buildings = client.buildings(&all).await?.items;

    match tactic {
        TacticId::RecoverColonyCrisis => {
            // 1. Reconnect unreachable sites with a Path chain from DistrictCenter
            if let Some(dc) = buildings
                .iter()
                .find(|b| b.finished == 1 && b.name.starts_with("DistrictCenter"))
            {
                if dc.paused == 1 {
                    client.pause_building(dc.id, false).await?;
                    steps.push(format!("unpause DistrictCenter {}", dc.id));
                }
                if dc.desired_workers.unwrap_or(0) < 2 {
                    client.set_workers(dc.id, 2).await?;
                    steps.push(format!("restore 2 builders on DistrictCenter {}", dc.id));
                }
                if let Some(target) = buildings
                    .iter()
                    .find(|b| b.reachable == Some(0) && !b.name.contains("Path"))
                {
                    let laid =
                        lay_line(client, "Path", (dc.x, dc.y), (target.x, target.y), 2, 16).await?;
                    if laid > 0 {
                        steps.push(format!(
                            "lay {laid} Path tiles from DC ({},{}) to unreachable {} ({},{})",
                            dc.x, dc.y, target.name, target.x, target.y
                        ));
                    }
                }
            }

            // 2. Bridge disconnected power consumers with PowerShafts from nearest generator
            if state.disconnected_power_consumers > 0 {
                let shaft_prefab = format!("PowerShaft.{suffix}");
                if let Some(consumer) = buildings.iter().find(|b| {
                    b.finished == 1
                        && b.paused == 0
                        && b.is_consumer == Some(1)
                        && b.powered == Some(0)
                }) {
                    if let Some(gen) = buildings
                        .iter()
                        .filter(|b| b.finished == 1 && b.paused == 0 && b.is_generator == Some(1))
                        .min_by_key(|g| (g.x - consumer.x).abs().max((g.y - consumer.y).abs()))
                    {
                        let laid = lay_line(
                            client,
                            &shaft_prefab,
                            (gen.x, gen.y),
                            (consumer.x, consumer.y),
                            3,
                            10,
                        )
                        .await?;
                        if laid > 0 {
                            steps.push(format!(
                                "bridge {} ({},{}) -> {} ({},{}) with {laid} {shaft_prefab}",
                                gen.name, gen.x, gen.y, consumer.name, consumer.x, consumer.y
                            ));
                        }
                    }
                }
            }

            // 3. Triage essential food & clean water buildings during dual collapse
            let clean_river =
                state.river_depth > 0.05 && state.river_contamination < IRRIGATION_POLLUTION_CUTOFF;
            for b in buildings.iter().filter(|b| {
                b.finished == 1
                    && (b.name.contains("FarmHouse")
                        || b.name.contains("Farmhouse")
                        || b.name.contains("GathererFlag")
                        || (clean_river && b.name.contains("WaterPump")))
            }) {
                if b.paused == 1 {
                    client.pause_building(b.id, false).await?;
                    steps.push(format!("unpause essential {} ({})", b.name, b.id));
                }
                if b.workplace_priority != Some(Priority::VeryHigh) {
                    client
                        .set_priority(&SetPriorityRequest {
                            id: b.id,
                            priority: Priority::VeryHigh,
                            priority_type: Some("workplace".to_string()),
                        })
                        .await?;
                    steps.push(format!(
                        "prioritize essential {} ({}) VeryHigh",
                        b.name, b.id
                    ));
                }
            }
            if (state.water_runway_hours < 36.0 && state.food_runway_hours < 36.0)
                && state.work_hours < 18
            {
                client.set_workhours(18).await?;
                steps.push("extend workhours to 18h for dual-crisis triage".to_string());
            }
        }

        TacticId::DivertBadtideFloodgates => {
            let badtide_now = state.weather_phase == WeatherPhase::Badtide
                || state.upstream_contamination >= 0.25
                || state.river_contamination >= IRRIGATION_POLLUTION_CUTOFF;

            // 1. Prioritize any unfinished floodgate & boost builders/workhours if badtide approaching
            for b in buildings
                .iter()
                .filter(|b| b.finished == 0 && b.name.contains("Floodgate"))
            {
                if b.paused == 1 {
                    client.pause_building(b.id, false).await?;
                    steps.push(format!("unpause floodgate site {}", b.id));
                }
                if b.construction_priority != Some(Priority::VeryHigh) {
                    client
                        .set_priority(&SetPriorityRequest {
                            id: b.id,
                            priority: Priority::VeryHigh,
                            priority_type: Some("construction".to_string()),
                        })
                        .await?;
                    steps.push(format!("prioritize floodgate {} VeryHigh", b.id));
                }
                if state.work_hours < 18 {
                    client.set_workhours(18).await?;
                    steps.push(
                        "extend workhours to 18h to finish floodgate before badtide".to_string(),
                    );
                }
                if let Some(dc) = buildings
                    .iter()
                    .find(|d| d.finished == 1 && d.name.starts_with("DistrictCenter"))
                {
                    if dc.desired_workers.unwrap_or(0) < 4 {
                        client.set_workers(dc.id, 4).await?;
                        steps.push(format!("assign 4 builders on DistrictCenter {}", dc.id));
                    }
                }
            }

            // 2. If no floodgate exists yet and we have prereqs, unlock + place one on the river
            if state.floodgates.is_empty() && state.logs >= 10 && state.planks >= 5 {
                let fg_prefab = format!("Floodgate.{suffix}");
                if !state.floodgate_unlocked && state.science >= 150 {
                    client.unlock_building(&fg_prefab).await?;
                    steps.push(format!("unlock {fg_prefab}"));
                }
                if let Some(id) = place_near(client, &fg_prefab, 10, 12, 6).await? {
                    client
                        .set_priority(&SetPriorityRequest {
                            id,
                            priority: Priority::VeryHigh,
                            priority_type: Some("construction".to_string()),
                        })
                        .await?;
                    steps.push(format!("place & prioritize {fg_prefab} ({id})"));
                }
            }

            // 3. Operate finished floodgates & water pumps according to contamination state
            if badtide_now
                || (state.weather_phase == WeatherPhase::BadtideApproaching
                    && state.days_until_hazard <= 1)
            {
                for b in buildings.iter().filter(|b| {
                    b.finished == 1 && (b.floodgate == Some(1) || b.name.contains("Floodgate"))
                }) {
                    let target = b.max_height.unwrap_or(1.0);
                    if b.height.unwrap_or(0.0) < target - 0.05 {
                        client.set_floodgate(b.id, target).await?;
                        steps.push(format!("raise floodgate {} to {:.2}", b.id, target));
                    }
                }
                if state.river_contamination >= IRRIGATION_POLLUTION_CUTOFF {
                    for b in buildings.iter().filter(|b| {
                        b.finished == 1 && b.paused == 0 && b.name.contains("WaterPump")
                    }) {
                        client.pause_building(b.id, true).await?;
                        steps.push(format!("pause contaminated pump {}", b.id));
                    }
                }
            } else if !state.weather_phase.is_badtide_like()
                && state.river_contamination < 0.15
                && state.upstream_contamination < 0.15
            {
                for b in buildings.iter().filter(|b| {
                    b.finished == 1 && (b.floodgate == Some(1) || b.name.contains("Floodgate"))
                }) {
                    if b.height.unwrap_or(0.0) > 0.55 {
                        client.set_floodgate(b.id, 0.5).await?;
                        steps.push(format!("lower floodgate {} to 0.50", b.id));
                    }
                }
                for b in buildings
                    .iter()
                    .filter(|b| b.finished == 1 && b.paused == 1 && b.name.contains("WaterPump"))
                {
                    client.pause_building(b.id, false).await?;
                    steps.push(format!("resume clean pump {}", b.id));
                }
                if state.work_hours > 16 {
                    client.set_workhours(16).await?;
                    steps.push("restore workhours to 16h".to_string());
                }
            }
        }

        TacticId::EmergencyFoodRationing => {
            // 1. Unpause and max-prioritize all farmhouses & gatherer flags
            for b in buildings.iter().filter(|b| {
                b.finished == 1
                    && (b.name.contains("FarmHouse")
                        || b.name.contains("Farmhouse")
                        || b.name.contains("GathererFlag"))
            }) {
                if b.paused == 1 {
                    client.pause_building(b.id, false).await?;
                    steps.push(format!("unpause food building {} ({})", b.name, b.id));
                }
                if b.workplace_priority != Some(Priority::VeryHigh) {
                    client
                        .set_priority(&SetPriorityRequest {
                            id: b.id,
                            priority: Priority::VeryHigh,
                            priority_type: Some("workplace".to_string()),
                        })
                        .await?;
                    steps.push(format!("prioritize {} ({}) VeryHigh", b.name, b.id));
                }
            }
            // 2. Extend workday to 18h during severe food crunch
            if state.food_runway_hours < 48.0 && state.work_hours < 18 {
                client.set_workhours(18).await?;
                steps.push("extend workhours to 18h during food crunch".to_string());
            }
            // 3. Place Farmhouse if none exists and logs permit
            if state.farms_active == 0 && state.farms_paused == 0 && state.logs >= 25 {
                let prefab = match state.faction {
                    Faction::Folktails => "EfficientFarmHouse.Folktails",
                    Faction::IronTeeth => "FarmHouse.IronTeeth",
                };
                if let Some(id) = place_near(client, prefab, 18, 16, 8).await? {
                    steps.push(format!("place {prefab} ({id})"));
                }
            }
            // 4. Plant fast faction crop if land is irrigated & crop count is low
            if state.irrigated_tiles > 0 && state.total_crops < 24 {
                let crop = match state.faction {
                    Faction::Folktails => "Carrot",
                    Faction::IronTeeth => "Kohlrabi",
                };
                client
                    .plant_crop(&PlantCropRequest {
                        crop: crop.to_string(),
                        x1: 18,
                        y1: 14,
                        x2: 20,
                        y2: 15,
                        z: 2,
                    })
                    .await?;
                steps.push(format!("plant fast {crop} patch at (18..20,14..15)"));
            }
        }

        TacticId::SecureEarlyWater => {
            // Unpause paused pumps if river is clean
            if state.river_contamination < IRRIGATION_POLLUTION_CUTOFF && state.river_depth > 0.05 {
                for b in buildings
                    .iter()
                    .filter(|b| b.finished == 1 && b.paused == 1 && b.name.contains("WaterPump"))
                {
                    client.pause_building(b.id, false).await?;
                    steps.push(format!("unpause pump {}", b.id));
                }
            }
            // Ensure at least one pump
            if state.pumps_active == 0 && state.pumps_paused == 0 && state.logs >= 12 {
                let pump = match state.faction {
                    Faction::Folktails => "WaterPump.Folktails",
                    Faction::IronTeeth => "DeepWaterPump.IronTeeth",
                };
                if let Some(id) = place_near(client, pump, 12, 14, 6).await? {
                    steps.push(format!("place {pump} ({id})"));
                }
            } else if state.logs >= 15 {
                let tank = format!("SmallTank.{suffix}");
                if let Some(id) = place_near(client, &tank, 14, 14, 8).await? {
                    steps.push(format!("place {tank} ({id})"));
                }
            }
        }

        TacticId::BuildDamBeforeDrought => {
            if let Some(dam) = buildings
                .iter()
                .find(|b| b.finished == 0 && b.name.starts_with("Dam."))
            {
                if dam.paused == 1 {
                    client.pause_building(dam.id, false).await?;
                    steps.push(format!("unpause Dam {}", dam.id));
                }
                client
                    .set_priority(&SetPriorityRequest {
                        id: dam.id,
                        priority: Priority::VeryHigh,
                        priority_type: Some("construction".to_string()),
                    })
                    .await?;
                steps.push(format!("prioritize Dam {} VeryHigh", dam.id));
            } else if state.dams_finished == 0 && state.logs >= 20 {
                let dam = format!("Dam.{suffix}");
                if let Some(id) = place_near(client, &dam, 10, 18, 8).await? {
                    client
                        .set_priority(&SetPriorityRequest {
                            id,
                            priority: Priority::VeryHigh,
                            priority_type: Some("construction".to_string()),
                        })
                        .await?;
                    steps.push(format!("place & prioritize {dam} ({id})"));
                }
            }
        }

        TacticId::StagePlankChain => {
            // Strict precedence: Power generator BEFORE LumberMill!
            let gen_prefab = match state.faction {
                Faction::Folktails => "PowerWheel.Folktails",
                Faction::IronTeeth => "LargePowerWheel.IronTeeth",
            };
            let gen_cost = match state.faction {
                Faction::Folktails => 10,
                Faction::IronTeeth => 25,
            };
            if state.power_generators_finished == 0 && state.power_generators_queued == 0 {
                if state.logs >= gen_cost {
                    if let Some(id) = place_near(client, gen_prefab, 17, 17, 8).await? {
                        client
                            .set_priority(&SetPriorityRequest {
                                id,
                                priority: Priority::High,
                                priority_type: Some("construction".to_string()),
                            })
                            .await?;
                        steps.push(format!(
                            "place power generator {gen_prefab} ({id}) before LumberMill"
                        ));
                    }
                }
            } else if state.lumber_mills_finished == 0 && state.lumber_mills_queued == 0 {
                let mill = format!("LumberMill.{suffix}");
                if state.logs >= 15 {
                    if let Some(id) = place_near(client, &mill, 18, 17, 8).await? {
                        steps.push(format!("place {mill} ({id})"));
                    }
                }
            } else {
                // Unpause existing power generators / lumber mills
                for b in buildings.iter().filter(|b| {
                    b.finished == 1
                        && b.paused == 1
                        && (b.is_generator == Some(1) || b.name.contains("LumberMill"))
                }) {
                    client.pause_building(b.id, false).await?;
                    steps.push(format!("unpause {} ({})", b.name, b.id));
                }
            }
        }

        TacticId::StageGearChain => {
            let gw_prefab = format!("GearWorkshop.{suffix}");
            let shaft_prefab = format!("PowerShaft.{suffix}");
            // 1. Ensure enough connected power supply (>= demand + 120 hp)
            if state.power_supply < state.power_demand + 120.0 {
                let gen_prefab = match state.faction {
                    Faction::Folktails => "WaterWheel.Folktails",
                    Faction::IronTeeth => "LargePowerWheel.IronTeeth",
                };
                let (gx, gy) = match state.faction {
                    Faction::Folktails => (10, 16),
                    Faction::IronTeeth => (17, 17),
                };
                if let Some(id) = place_near(client, gen_prefab, gx, gy, 6).await? {
                    client
                        .set_priority(&SetPriorityRequest {
                            id,
                            priority: Priority::High,
                            priority_type: Some("construction".to_string()),
                        })
                        .await?;
                    steps.push(format!(
                        "place power generator {gen_prefab} ({id}) for gears"
                    ));
                    // If Folktails WaterWheel on river x=10, lay PowerShafts toward workshop hub (17,16)
                    if state.faction == Faction::Folktails {
                        let laid =
                            lay_line(client, &shaft_prefab, (10, 16), (17, 16), 3, 4).await?;
                        if laid > 0 {
                            steps.push(format!(
                                "lay {laid} {shaft_prefab} from river WaterWheel to hub"
                            ));
                        }
                    }
                }
            } else {
                // 2. Unlock & place GearWorkshop adjacent to an existing power generator
                if !state.gear_workshop_unlocked && state.science >= 100 {
                    client.unlock_building(&gw_prefab).await?;
                    steps.push(format!("unlock {gw_prefab} (100 SP)"));
                }
                if state.gear_workshops_finished == 0
                    && state.gear_workshops_queued == 0
                    && state.logs >= 15
                    && state.planks >= 25
                {
                    let (ax, ay) = buildings
                        .iter()
                        .find(|b| b.finished == 1 && b.is_generator == Some(1) && b.x >= 13)
                        .or_else(|| {
                            buildings
                                .iter()
                                .find(|b| b.finished == 1 && b.name.contains("PowerShaft"))
                        })
                        .map(|b| (b.x, b.y))
                        .unwrap_or((17, 17));
                    if let Some(id) = place_near(client, &gw_prefab, ax, ay, 3).await? {
                        client
                            .set_priority(&SetPriorityRequest {
                                id,
                                priority: Priority::High,
                                priority_type: Some("construction".to_string()),
                            })
                            .await?;
                        steps.push(format!(
                            "place & prioritize {gw_prefab} ({id}) near power hub ({ax},{ay})"
                        ));
                    } else if let Some(id) = place_near(client, &gw_prefab, 18, 17, 6).await? {
                        steps.push(format!("place {gw_prefab} ({id})"));
                    }
                }
            }
        }

        TacticId::ManageBreedingPods => {
            let should_pause = state.free_beds <= 0
                || state.berries < 15
                || state.water_runway_hours < 60.0
                || state.food_runway_hours < 60.0
                || state.weather_phase.is_hazardous();
            if should_pause {
                for b in buildings.iter().filter(|b| {
                    b.finished == 1 && b.paused == 0 && b.name.starts_with("BreedingPod")
                }) {
                    client.pause_building(b.id, true).await?;
                    steps.push(format!(
                        "pause BreedingPod {} to protect beds/berries/runway",
                        b.id
                    ));
                }
            } else {
                let mut resumed = false;
                for b in buildings.iter().filter(|b| {
                    b.finished == 1 && b.paused == 1 && b.name.starts_with("BreedingPod")
                }) {
                    client.pause_building(b.id, false).await?;
                    steps.push(format!(
                        "resume BreedingPod {} with healthy runways & free beds",
                        b.id
                    ));
                    resumed = true;
                }
                if !resumed
                    && state.breeding_pods_active == 0
                    && state.breeding_pods_queued == 0
                    && state.logs >= 20
                {
                    if let Some(id) = place_near(client, "BreedingPod.IronTeeth", 17, 18, 8).await?
                    {
                        steps.push(format!("place BreedingPod.IronTeeth ({id})"));
                    }
                }
            }
        }

        TacticId::UnlockAndPlantForester => {
            let forester = format!("Forester.{suffix}");
            if !state.forester_unlocked && state.science >= 30 {
                client.unlock_building(&forester).await?;
                steps.push(format!("unlock {forester} (30 SP)"));
            }
            if state.foresters_finished == 0 && state.foresters_queued == 0 {
                if let Some(id) = place_near(client, &forester, 21, 15, 10).await? {
                    client
                        .set_priority(&SetPriorityRequest {
                            id,
                            priority: Priority::High,
                            priority_type: Some("construction".to_string()),
                        })
                        .await?;
                    steps.push(format!("place & prioritize {forester} ({id})"));
                }
            }
            // Mark surrounding forest area for harvesting once grown
            client
                .mark_trees(&CuttingAreaRequest {
                    x1: 20,
                    y1: 10,
                    x2: 28,
                    y2: 20,
                    z: 2,
                    marked: true,
                })
                .await?;
            steps.push("mark forestry cutting area (20..28, 10..20)".to_string());
        }

        TacticId::AdvanceScienceAndQueue => {
            if state.active_builders == 0 {
                for dc in buildings
                    .iter()
                    .filter(|b| b.finished == 1 && b.name.starts_with("DistrictCenter"))
                {
                    if dc.paused == 1 {
                        client.pause_building(dc.id, false).await?;
                        steps.push(format!("unpause DistrictCenter {}", dc.id));
                    }
                    client.set_workers(dc.id, 2).await?;
                    steps.push(format!("assign 2 builders on DistrictCenter {}", dc.id));
                }
            }
            if state.inventors_finished == 0 && state.logs >= 12 {
                let inv = format!("Inventor.{suffix}");
                if let Some(id) = place_near(client, &inv, 15, 15, 8).await? {
                    steps.push(format!("place {inv} ({id})"));
                }
            }
        }

        TacticId::ExpandHousingCapped => {
            let house = match state.faction {
                Faction::Folktails => "Lodge.Folktails",
                Faction::IronTeeth => "Barrack.IronTeeth",
            };
            if let Some(id) = place_near(client, house, 18, 19, 10).await? {
                steps.push(format!("place {house} ({id})"));
            }
        }

        TacticId::HoldAndMonitor => {
            return Ok((OutcomeStatus::NoOp, steps));
        }
    }

    let status = if steps.is_empty() {
        OutcomeStatus::NoOp
    } else {
        OutcomeStatus::Succeeded
    };
    Ok((status, steps))
}

async fn place_near(
    client: &TimberbotClient,
    prefab: &str,
    x: i32,
    y: i32,
    radius: i32,
) -> Result<Option<i64>> {
    let resp = client
        .find_placement(&FindPlacementRequest {
            prefab: prefab.to_string(),
            x: Some(x),
            y: Some(y),
            radius: Some(radius),
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 0,
        })
        .await?;
    let Some(c) = resp.candidates.first() else {
        return Ok(None);
    };
    let placed = client
        .place_building(&PlaceBuildingRequest {
            prefab: prefab.to_string(),
            x: c.x,
            y: c.y,
            z: c.z,
            orientation: Orientation::South,
        })
        .await?;
    Ok(placed.get("id").and_then(|v| v.as_i64()))
}

/// Lay a deterministic stepped line of `Path` or `PowerShaft` tiles from `from` to `to`.
async fn lay_line(
    client: &TimberbotClient,
    prefab: &str,
    from: (i32, i32),
    to: (i32, i32),
    step: i32,
    max_tiles: usize,
) -> Result<usize> {
    let mut laid = 0usize;
    let (mut cx, mut cy) = from;
    let (x1, y1) = to;
    let step = step.max(1);

    for _ in 0..max_tiles {
        let dx = x1 - cx;
        let dy = y1 - cy;
        if dx.abs() <= step && dy.abs() <= step {
            break;
        }
        if dx.abs() >= dy.abs() {
            cx += dx.signum() * dx.abs().min(step);
        } else {
            cy += dy.signum() * dy.abs().min(step);
        }
        let req = PlaceBuildingRequest {
            prefab: prefab.to_string(),
            x: cx,
            y: cy,
            z: 2,
            orientation: Orientation::South,
        };
        if client.place_building(&req).await.is_ok()
            || place_near(client, prefab, cx, cy, 1).await?.is_some()
        {
            laid += 1;
        }
    }
    Ok(laid)
}

/// Convenience helper combining Layer 1 (`ModSnapshot::fetch` -> `ColonyState`) and Layer 2
/// (`decide` -> `execute_tactic`), returning `(state, decision, outcome, next_commitment)`.
pub async fn run_four_layer_tick(
    client: &TimberbotClient,
    active_commitment: Option<Commitment>,
) -> Result<(ColonyState, Decision, PlaybookOutcome, Option<Commitment>)> {
    let snap = ModSnapshot::fetch(client).await?;
    let state = ColonyState::from_snapshot(&snap, active_commitment);
    let decision = decide(&state);
    let outcome = execute_tactic(client, &state, &decision).await;
    let next_commitment = match outcome.status {
        OutcomeStatus::Failed => None,
        _ => decision.commitment.clone(),
    };
    Ok((state, decision, outcome, next_commitment))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use timberbot_state::{ConstructionEta, FloodgateSummary};

    fn healthy_state() -> ColonyState {
        ColonyState {
            faction: Faction::Folktails,
            day: 5,
            cycle: 1,
            cycle_day: 5,
            work_hours: 16,
            active_builders: 2,
            weather_phase: WeatherPhase::Temperate,
            days_until_hazard: 10,
            hazard_duration_days: 6,
            river_depth: 1.5,
            river_contamination: 0.0,
            upstream_contamination: 0.0,
            irrigated_tiles: 40,
            contaminated_tiles: 0,
            population: 12,
            adults: 9,
            children: 3,
            water_stock: 220,
            water_capacity: 330,
            water_runway_hours: 207.5,
            food_stock: 200,
            food_runway_hours: 150.9,
            logs: 60,
            planks: 20,
            gears: 0,
            treated_planks: 0,
            metal_blocks: 0,
            berries: 40,
            science: 200,
            total_beds: 15,
            free_beds: 3,
            unemployed_adults: 2,
            worker_shortage: 0,
            idle_workers: 2,
            contaminated_beavers: 0,
            pumps_active: 1,
            pumps_paused: 0,
            farms_active: 1,
            farms_paused: 0,
            foresters_finished: 1,
            foresters_queued: 0,
            forester_unlocked: true,
            dams_finished: 1,
            dams_queued: 0,
            floodgate_unlocked: true,
            floodgates: vec![FloodgateSummary {
                id: 1010,
                x: 10,
                y: 12,
                finished: true,
                height: 0.5,
                max_height: 1.0,
            }],
            lumber_mills_finished: 1,
            lumber_mills_queued: 0,
            lumber_mills_unpowered: 0,
            power_generators_finished: 1,
            power_generators_queued: 0,
            inventors_finished: 1,
            power_supply: 50.0,
            power_demand: 50.0,
            grown_trees: 45,
            sapling_trees: 10,
            total_crops: 30,
            grown_crops: 10,
            breeding_pods_active: 0,
            breeding_pods_paused: 0,
            breeding_pods_queued: 0,
            gear_workshops_finished: 0,
            gear_workshops_queued: 0,
            gear_workshop_unlocked: false,
            unreachable_buildings: 0,
            disconnected_power_consumers: 0,
            power_network_count: 1,
            wellbeing_avg: 8.0,
            construction_queue: Vec::new(),
            blocked_prerequisites: Vec::new(),
            active_commitment: None,
        }
    }

    #[test]
    fn badtide_2d_with_unbuilt_floodgate_picks_divert_tactic() {
        let mut s = healthy_state();
        s.weather_phase = WeatherPhase::BadtideApproaching;
        s.days_until_hazard = 2;
        s.floodgates = vec![FloodgateSummary {
            id: 1020,
            x: 10,
            y: 12,
            finished: false,
            height: 0.0,
            max_height: 1.0,
        }];
        s.construction_queue = vec![ConstructionEta {
            id: 1020,
            name: "Floodgate.Folktails".to_string(),
            priority: Priority::Normal,
            paused: false,
            material_progress: 0.5,
            build_progress: 0.0,
            remaining_work_hours: 8.6,
            eta_hours: Some(4.3),
            eta_days: Some(0.27),
        }];
        let d = decide(&s);
        assert_eq!(d.tactic, TacticId::DivertBadtideFloodgates);
        assert!(d.commitment.is_some());
    }

    #[test]
    fn low_logs_no_forester_enforces_plank_prerequisite_then_forester() {
        // Without planks, must stage_plank_chain first (prerequisite check!)
        let mut s = healthy_state();
        s.logs = 30;
        s.grown_trees = 8;
        s.planks = 0;
        s.foresters_finished = 0;
        s.forester_unlocked = false;
        s.science = 40;
        s.lumber_mills_finished = 0;
        s.power_generators_finished = 0;
        assert_eq!(decide(&s).tactic, TacticId::StagePlankChain);

        // Once planks >= 7 exist, unlocks and plants Forester!
        s.planks = 10;
        s.lumber_mills_finished = 1;
        s.power_generators_finished = 1;
        assert_eq!(decide(&s).tactic, TacticId::UnlockAndPlantForester);
    }

    #[test]
    fn mid_drought_food_crunch_picks_emergency_food_rationing() {
        let mut s = healthy_state();
        s.weather_phase = WeatherPhase::Drought;
        s.days_until_hazard = 0;
        s.river_depth = 0.0;
        s.food_stock = 30;
        s.food_runway_hours = 22.6;
        s.farms_active = 0;
        s.farms_paused = 1;
        assert_eq!(decide(&s).tactic, TacticId::EmergencyFoodRationing);
    }

    #[test]
    fn lead_time_check_prevents_infeasible_dam_right_before_drought() {
        let mut s = healthy_state();
        s.dams_finished = 0;
        s.floodgates.clear();
        s.weather_phase = WeatherPhase::DroughtApproaching;
        s.days_until_hazard = 0; // 0d left -> 0.30d Dam lead time does NOT fit
        let cat = available_catalogue(&s);
        let dam = cat
            .iter()
            .find(|e| e.tactic == TacticId::BuildDamBeforeDrought)
            .unwrap();
        assert!(!dam.feasible_before_hazard);
        assert!(!dam.enabled);
    }

    #[test]
    fn midgame_gear_chain_picks_stage_gear_chain_when_planks_and_science_ready() {
        let mut s = healthy_state();
        s.planks = 30;
        s.logs = 50;
        s.gears = 0;
        s.science = 150;
        s.power_supply = 200.0;
        s.power_demand = 50.0;
        assert_eq!(decide(&s).tactic, TacticId::StageGearChain);
    }

    #[test]
    fn iron_teeth_breeding_pod_throttles_when_beds_or_berries_tight() {
        let mut s = healthy_state();
        s.faction = Faction::IronTeeth;
        s.breeding_pods_active = 2;
        s.free_beds = 0;
        s.berries = 10;
        assert_eq!(decide(&s).tactic, TacticId::ManageBreedingPods);

        // Resumes paused pod when beds & berries are healthy
        s.breeding_pods_active = 0;
        s.breeding_pods_paused = 1;
        s.free_beds = 4;
        s.berries = 45;
        assert_eq!(decide(&s).tactic, TacticId::ManageBreedingPods);
    }

    #[test]
    fn crisis_recovery_handles_unreachable_sites_and_disconnected_power() {
        let mut s = healthy_state();
        s.unreachable_buildings = 1;
        assert_eq!(decide(&s).tactic, TacticId::RecoverColonyCrisis);

        let mut s2 = healthy_state();
        s2.disconnected_power_consumers = 1;
        assert_eq!(decide(&s2).tactic, TacticId::RecoverColonyCrisis);
    }
}
