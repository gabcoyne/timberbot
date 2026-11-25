//! Verified Timberborn 1.1 game data tables, difficulty presets, and time/precedence constants.
//!
//! Every constant here traces back to `docs/mock-fidelity.md`:
//! - Official Timberborn Wiki (`timberborn.wiki.gg`)
//! - Recorded live-mod fixtures (`python/tests/fixtures/openapi/prefabs.json`, `wellbeing.json`)
//! - Timberborn Update 1.1 Patch Notes (Sept 3, 2026)
//!
//! Constants explicitly marked `approximated` (e.g. per-building construction work-hours and
//! 1D river wave propagation speed) are deterministic offline approximations documented in
//! `docs/mock-fidelity.md`.

use serde::{Deserialize, Serialize};
use timberbot_protocol::http::{PrefabCost, PrefabInfo};

/// Game difficulty preset (verified: `timberborn.wiki.gg/wiki/Game_Mode`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

/// Verified parameters per difficulty mode (`timberborn.wiki.gg/wiki/Game_Mode`).
#[derive(Debug, Clone, Copy)]
pub struct DifficultySpec {
    pub starting_adults: u32,
    pub starting_children: u32,
    pub starting_food: i64,
    pub starting_water: i64,
    pub consumption_mult: f64,
    pub temperate_min_days: u32,
    pub temperate_max_days: u32,
    pub drought_min_days: u32,
    pub drought_max_days: u32,
    pub drought_handicap_start: f64,
    pub drought_handicap_cycles: u32,
    pub badtide_min_days: u32,
    pub badtide_max_days: u32,
    pub badtide_chance: f64,
    pub badtide_min_cycle: u32,
    pub badtide_handicap_start: f64,
    pub badtide_handicap_cycles: u32,
    pub demolition_refund: f64,
}

impl Difficulty {
    pub const fn spec(self) -> DifficultySpec {
        match self {
            Difficulty::Easy => DifficultySpec {
                starting_adults: 9,
                starting_children: 4,
                starting_food: 300,
                starting_water: 250,
                consumption_mult: 0.40,
                temperate_min_days: 16,
                temperate_max_days: 19,
                drought_min_days: 2,
                drought_max_days: 4,
                drought_handicap_start: 0.25,
                drought_handicap_cycles: 8,
                badtide_min_days: 1,
                badtide_max_days: 3,
                badtide_chance: 0.40,
                badtide_min_cycle: 5,
                badtide_handicap_start: 0.30,
                badtide_handicap_cycles: 6,
                demolition_refund: 0.90,
            },
            Difficulty::Normal => DifficultySpec {
                starting_adults: 9,
                starting_children: 4,
                starting_food: 130,
                starting_water: 0,
                consumption_mult: 1.00,
                temperate_min_days: 13,
                temperate_max_days: 17,
                drought_min_days: 5,
                drought_max_days: 9,
                drought_handicap_start: 0.38,
                drought_handicap_cycles: 5,
                badtide_min_days: 4,
                badtide_max_days: 8,
                badtide_chance: 0.40,
                badtide_min_cycle: 4,
                badtide_handicap_start: 0.15,
                badtide_handicap_cycles: 5,
                demolition_refund: 0.75,
            },
            Difficulty::Hard => DifficultySpec {
                starting_adults: 9,
                starting_children: 4,
                starting_food: 90,
                starting_water: 0,
                consumption_mult: 1.00,
                temperate_min_days: 5,
                temperate_max_days: 8,
                drought_min_days: 15,
                drought_max_days: 30,
                drought_handicap_start: 0.20,
                drought_handicap_cycles: 12,
                badtide_min_days: 15,
                badtide_max_days: 30,
                badtide_chance: 0.40,
                badtide_min_cycle: 3,
                badtide_handicap_start: 0.40,
                badtide_handicap_cycles: 9,
                demolition_refund: 0.75,
            },
        }
    }

    /// Deterministic temperate duration (midpoint of verified range).
    pub const fn nominal_temperate_days(self) -> u32 {
        let s = self.spec();
        (s.temperate_min_days + s.temperate_max_days) / 2
    }

    /// Deterministic hazardous duration for a given 1-based cycle with verified handicap ramp.
    pub fn hazard_duration_for_cycle(self, cycle: u32, is_badtide: bool) -> u32 {
        let s = self.spec();
        let (min_d, max_d, h_start, h_cycles) = if is_badtide {
            (
                s.badtide_min_days,
                s.badtide_max_days,
                s.badtide_handicap_start,
                s.badtide_handicap_cycles,
            )
        } else {
            (
                s.drought_min_days,
                s.drought_max_days,
                s.drought_handicap_start,
                s.drought_handicap_cycles,
            )
        };
        let base = (min_d + max_d) as f64 / 2.0;
        let idx = cycle.saturating_sub(1);
        let mult = if h_cycles == 0 || idx >= h_cycles {
            1.0
        } else {
            h_start + (1.0 - h_start) * (idx as f64 / h_cycles as f64)
        };
        (base * mult).round().max(1.0) as u32
    }
}

// --- Verified Needs & Consumption (`timberborn.wiki.gg/wiki/Needs`) ---

/// Base food consumption per organic beaver per day (`80/day` decay, `+30/food` -> `2.67 food/day`).
pub const FOOD_PER_BEAVER_DAY: f64 = 80.0 / 30.0;
/// Base water consumption per organic beaver per day (`70/day` decay, `+33/water` -> `2.12 water/day`).
pub const WATER_PER_BEAVER_DAY: f64 = 70.0 / 33.0;
/// Biofuel consumption per Folktails bot per day (`58/day` decay, `+30/biofuel` -> `1.93/day`).
pub const BIOFUEL_PER_BOT_DAY: f64 = 58.0 / 30.0;
/// Days at 0 hunger (starving) before death (`300 / 80 = 3.75 days`).
pub const STARVATION_DEATH_DAYS: f64 = 3.75;
/// Days at 0 thirst (thirsty) before death (`300 / 70 = 4.2857 days`).
pub const DEHYDRATION_DEATH_DAYS: f64 = 300.0 / 70.0;
/// Days of badwater exposure before Unwell transitions to Contaminated (`3.0 days`).
pub const CONTAMINATION_ONSET_DAYS: f64 = 3.0;

// --- Verified Water, Evaporation & Pumping (`Water_Pump`, `Evaporation`, `Dam`, `Floodgate`) ---

/// Water Pump / Deep Water Pump verified output: `0.33h` per 1 Water = `3.0 Water/hour` per worker.
pub const WATER_PUMP_PER_HOUR: f64 = 3.0;
/// Large Water Pump verified output: 5x standard pump = `15.0 Water/hour`.
pub const LARGE_WATER_PUMP_PER_HOUR: f64 = 15.0;
/// Dam spill height (verified: `0.65` blocks).
pub const DAM_SPILL_HEIGHT: f64 = 0.65;
/// Evaporation rate for a 3+ tile wide reservoir (`0.04608 * 1.1600 = 0.0534 m/day`).
pub const EVAP_WIDE_POOL_PER_DAY: f64 = 0.0534;
/// Evaporation rate for a 2-tile wide channel (`0.04608 * 2.0835 = 0.096 m/day`).
pub const EVAP_MEDIUM_CHANNEL_PER_DAY: f64 = 0.096;
/// Evaporation rate for an isolated 1-tile wide channel (`0.04608 * 4.3405 = 0.200 m/day`).
pub const EVAP_NARROW_CHANNEL_PER_DAY: f64 = 0.200;
/// Pollution threshold at or above which river tiles cease irrigating adjacent land (`50%`).
pub const IRRIGATION_POLLUTION_CUTOFF: f64 = 0.50;

// --- Approximated Time-Lag Constants (documented as `approximated` in `docs/mock-fidelity.md`) ---

/// Approximated downstream river wave propagation speed (`tiles/day`, 1 tile/game-hour) across the 1D mock channel.
pub const WATER_PROPAGATION_TILES_PER_DAY: f64 = 24.0;
/// Approximated construction material hauling rate (`items / builder-hour` at base distance <= 6).
pub const HAUL_ITEMS_PER_BUILDER_HOUR: f64 = 12.0;

// --- Verified & Approximated Production & Reproduction ---

/// Inventor verified output: `1.0` Science Point per working hour (`wiki Inventor`).
pub const INVENTOR_SP_PER_HOUR: f64 = 1.0;
/// Lumber Mill verified output: `1 Plank / 1.3h` = `0.7692 Plank/hour` (`wiki Lumber_Mill`).
pub const LUMBER_MILL_PLANKS_PER_HOUR: f64 = 1.0 / 1.3;
/// Gear Workshop approximated output: `1 Gear / 2.2h` (consumes 1 Plank per Gear).
pub const GEAR_WORKSHOP_GEARS_PER_HOUR: f64 = 1.0 / 2.2;
/// Wood Workshop approximated output: `1 TreatedPlank / 2.5h` (consumes 1 Plank + 1 PineResin).
pub const WOOD_WORKSHOP_TREATED_PER_HOUR: f64 = 1.0 / 2.5;
/// Smelter approximated output: `1 MetalBlock / 3.0h` (consumes 2 ScrapMetal).
pub const SMELTER_METAL_PER_HOUR: f64 = 1.0 / 3.0;
/// Forester approximated planting cadence: `~0.5` saplings per working hour.
pub const FORESTER_PLANT_PER_HOUR: f64 = 0.5;
/// Fermenter verified output (Iron Teeth): `4 Cassava -> 10 FermentedCassava / 2h` = `5.0/hour`.
pub const FERMENTER_OUTPUT_PER_HOUR: f64 = 5.0;
/// Breeding Pod verified cycle (Iron Teeth): `5 Water + 5 Berries` over `5.0 days` -> 1 kit.
pub const BREEDING_POD_CYCLE_DAYS: f64 = 5.0;
pub const BREEDING_POD_WATER_COST: f64 = 5.0;
pub const BREEDING_POD_BERRIES_COST: f64 = 5.0;

/// Verified crop growth duration (days) and harvest yield per tile (`timberborn.wiki.gg/wiki/Crops`).
pub fn crop_spec(crop: &str) -> (f64, f64) {
    match crop {
        "Carrot" => (4.0, 3.0),    // 0.75/day (Folktails)
        "Sunflower" => (5.0, 2.0), // 0.40/day (Folktails)
        "Potato" => (6.0, 1.0),    // 1 -> 4 GrilledPotato
        "Wheat" => (10.0, 3.0),    // 0.30/day
        "Kohlrabi" => (3.0, 2.0),  // 0.67/day (Iron Teeth)
        "Cassava" => (5.0, 1.0),   // 4 -> 10 FermentedCassava (Iron Teeth)
        "Soybean" => (6.0, 1.0),   // Iron Teeth
        "Corn" => (10.0, 2.0),     // Iron Teeth
        _ => (4.0, 2.0),
    }
}

/// Verified tree growth duration (days) and log yield (`timberborn.wiki.gg/wiki/Trees`).
pub fn tree_spec(tree: &str) -> (f64, f64) {
    match tree {
        "Birch" => (9.0, 1.0),
        "Pine" => (12.0, 2.0),
        "Oak" => (30.0, 8.0),
        "Maple" => (24.0, 6.0),
        "Chestnut" => (24.0, 4.0),
        _ => (12.0, 2.0),
    }
}

/// Walking/hauling efficiency multiplier as a function of Manhattan distance from District Center
/// (`approximated` in `docs/mock-fidelity.md`). Tiles within 6 tiles incur no penalty.
pub fn distance_efficiency(manhattan_dist: i32) -> f64 {
    let extra = (manhattan_dist - 6).max(0) as f64;
    1.0 / (1.0 + extra * 0.025)
}

/// Verified building metadata table (costs, footprints, workers, housing, storage, power)
/// plus approximated `build_work_hours` for non-instant builder-queued construction.
#[derive(Debug, Clone)]
pub struct BuildingSpec {
    pub name: &'static str,
    pub size_x: u32,
    pub size_y: u32,
    pub size_z: u32,
    pub science_cost: u32,
    pub unlocked_default: bool,
    pub costs: &'static [(&'static str, u32)],
    pub max_workers: u32,
    pub max_dwellers: u32,
    pub storage_capacity: Option<i64>,
    pub max_floodgate_height: Option<f64>,
    pub power_output: f64,
    pub power_input: f64,
    pub is_wonder: bool,
    pub effect_radius: Option<i32>,
    /// Builder work-hours required after materials are hauled (`0.0` only for instant Path/Flags).
    /// Marked `approximated` in `docs/mock-fidelity.md`.
    pub build_work_hours: f64,
}

macro_rules! spec {
    (
        name: $name:expr, size: ($sx:expr, $sy:expr, $sz:expr),
        sp: $sp:expr, unlocked: $un:expr, costs: $costs:expr,
        workers: $w:expr, beds: $b:expr, cap: $cap:expr, fg: $fg:expr,
        p_out: $po:expr, p_in: $pi:expr, build_h: $bh:expr
    ) => {
        BuildingSpec {
            name: $name,
            size_x: $sx,
            size_y: $sy,
            size_z: $sz,
            science_cost: $sp,
            unlocked_default: $un,
            costs: $costs,
            max_workers: $w,
            max_dwellers: $b,
            storage_capacity: $cap,
            max_floodgate_height: $fg,
            power_output: $po,
            power_input: $pi,
            is_wonder: false,
            effect_radius: None,
            build_work_hours: $bh,
        }
    };
}

pub const BUILDING_SPECS: &[BuildingSpec] = &[
    // --- Shared ---
    spec!(name: "Path", size: (1,1,1), sp: 0, unlocked: true, costs: &[], workers: 0, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "Path.Folktails", size: (1,1,1), sp: 0, unlocked: true, costs: &[], workers: 0, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "Path.IronTeeth", size: (1,1,1), sp: 0, unlocked: true, costs: &[], workers: 0, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    // --- Folktails (costs/sizes/SP verified against `prefabs.json` & wiki) ---
    spec!(name: "DistrictCenter.Folktails", size: (3,3,5), sp: 0, unlocked: true, costs: &[], workers: 4, beds: 0, cap: Some(300), fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "BuildersHut.Folktails", size: (3,2,2), sp: 100, unlocked: false, costs: &[("Log", 20), ("Plank", 10)], workers: 4, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "WaterPump.Folktails", size: (2,3,2), sp: 0, unlocked: true, costs: &[("Log", 12)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 6.0),
    spec!(name: "SmallTank.Folktails", size: (1,1,2), sp: 0, unlocked: true, costs: &[("Log", 15)], workers: 0, beds: 0, cap: Some(30), fg: None, p_out: 0.0, p_in: 0.0, build_h: 4.0),
    spec!(name: "MediumTank.Folktails", size: (2,2,3), sp: 120, unlocked: false, costs: &[("Plank", 30), ("Gear", 20)], workers: 0, beds: 0, cap: Some(300), fg: None, p_out: 0.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "LargeTank.Folktails", size: (3,3,4), sp: 600, unlocked: false, costs: &[("Plank", 80), ("Gear", 60), ("MetalBlock", 30)], workers: 0, beds: 0, cap: Some(1200), fg: None, p_out: 0.0, p_in: 0.0, build_h: 24.0),
    spec!(name: "EfficientFarmHouse.Folktails", size: (2,3,2), sp: 0, unlocked: true, costs: &[("Log", 25)], workers: 2, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 8.0),
    // Alias kept for compatibility with Phase 1 scenario JSON files
    spec!(name: "Farmhouse.Folktails", size: (2,3,2), sp: 0, unlocked: true, costs: &[("Log", 25)], workers: 2, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "GathererFlag.Folktails", size: (1,1,1), sp: 0, unlocked: true, costs: &[], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "LumberjackFlag.Folktails", size: (1,1,2), sp: 0, unlocked: true, costs: &[], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "Forester.Folktails", size: (2,2,4), sp: 30, unlocked: false, costs: &[("Log", 10), ("Plank", 7)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 10.0),
    spec!(name: "Lodge.Folktails", size: (2,2,1), sp: 0, unlocked: true, costs: &[("Log", 12)], workers: 0, beds: 3, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 6.0),
    spec!(name: "MiniLodge.Folktails", size: (1,2,1), sp: 50, unlocked: false, costs: &[("Log", 5)], workers: 0, beds: 1, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 4.0),
    spec!(name: "DoubleLodge.Folktails", size: (2,2,2), sp: 150, unlocked: false, costs: &[("Log", 20)], workers: 0, beds: 6, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 10.0),
    spec!(name: "TripleLodge.Folktails", size: (3,2,2), sp: 250, unlocked: false, costs: &[("Log", 35)], workers: 0, beds: 9, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 14.0),
    spec!(name: "Dam.Folktails", size: (1,1,1), sp: 0, unlocked: true, costs: &[("Log", 20)], workers: 0, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "Floodgate.Folktails", size: (1,1,3), sp: 150, unlocked: false, costs: &[("Log", 10), ("Plank", 5)], workers: 0, beds: 0, cap: None, fg: Some(1.0), p_out: 0.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "DoubleFloodgate.Folktails", size: (1,1,2), sp: 250, unlocked: false, costs: &[("Log", 15), ("Plank", 10)], workers: 0, beds: 0, cap: None, fg: Some(2.0), p_out: 0.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "TripleFloodgate.Folktails", size: (1,1,3), sp: 500, unlocked: false, costs: &[("Log", 20), ("Plank", 15)], workers: 0, beds: 0, cap: None, fg: Some(3.0), p_out: 0.0, p_in: 0.0, build_h: 16.0),
    spec!(name: "Inventor.Folktails", size: (2,2,2), sp: 0, unlocked: true, costs: &[("Log", 12)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 6.0),
    spec!(name: "PowerWheel.Folktails", size: (3,1,2), sp: 0, unlocked: true, costs: &[("Log", 10)], workers: 1, beds: 0, cap: None, fg: None, p_out: 50.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "WaterWheel.Folktails", size: (3,1,2), sp: 0, unlocked: true, costs: &[("Log", 50)], workers: 0, beds: 0, cap: None, fg: None, p_out: 180.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "LumberMill.Folktails", size: (2,3,3), sp: 0, unlocked: true, costs: &[("Log", 15)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 50.0, build_h: 8.0),
    spec!(name: "GearWorkshop.Folktails", size: (2,3,2), sp: 100, unlocked: false, costs: &[("Log", 15), ("Plank", 25)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 120.0, build_h: 12.0),
    spec!(name: "WoodWorkshop.Folktails", size: (2,4,4), sp: 800, unlocked: false, costs: &[("Log", 20), ("Plank", 40), ("Gear", 40)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 200.0, build_h: 18.0),
    spec!(name: "Smelter.Folktails", size: (2,4,3), sp: 300, unlocked: false, costs: &[("Plank", 50), ("Gear", 20), ("ScrapMetal", 30)], workers: 2, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 200.0, build_h: 18.0),
    // --- Iron Teeth (costs/sizes verified against `timberborn.wiki.gg`) ---
    spec!(name: "DistrictCenter.IronTeeth", size: (3,3,5), sp: 0, unlocked: true, costs: &[], workers: 4, beds: 0, cap: Some(300), fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "BuildersHut.IronTeeth", size: (3,2,2), sp: 100, unlocked: false, costs: &[("Log", 20), ("Plank", 10)], workers: 4, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "DeepWaterPump.IronTeeth", size: (2,3,2), sp: 0, unlocked: true, costs: &[("Log", 12)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 6.0),
    spec!(name: "SmallTank.IronTeeth", size: (1,1,2), sp: 0, unlocked: true, costs: &[("Log", 15)], workers: 0, beds: 0, cap: Some(30), fg: None, p_out: 0.0, p_in: 0.0, build_h: 4.0),
    spec!(name: "MediumTank.IronTeeth", size: (2,2,3), sp: 120, unlocked: false, costs: &[("Plank", 30), ("Gear", 20)], workers: 0, beds: 0, cap: Some(300), fg: None, p_out: 0.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "FarmHouse.IronTeeth", size: (2,3,2), sp: 0, unlocked: true, costs: &[("Log", 25)], workers: 2, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "Fermenter.IronTeeth", size: (2,2,2), sp: 0, unlocked: true, costs: &[("Log", 25)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 50.0, build_h: 8.0),
    spec!(name: "GathererFlag.IronTeeth", size: (1,1,1), sp: 0, unlocked: true, costs: &[], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "LumberjackFlag.IronTeeth", size: (1,1,2), sp: 0, unlocked: true, costs: &[], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 0.0),
    spec!(name: "Forester.IronTeeth", size: (2,2,4), sp: 30, unlocked: false, costs: &[("Log", 10), ("Plank", 7)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 10.0),
    spec!(name: "Barrack.IronTeeth", size: (3,2,2), sp: 0, unlocked: true, costs: &[("Log", 40)], workers: 0, beds: 10, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "Rowhouse.IronTeeth", size: (2,1,3), sp: 180, unlocked: false, costs: &[("Log", 20)], workers: 0, beds: 5, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "BreedingPod.IronTeeth", size: (1,1,1), sp: 0, unlocked: true, costs: &[("Log", 10)], workers: 0, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 5.0),
    spec!(name: "Dam.IronTeeth", size: (1,1,1), sp: 0, unlocked: true, costs: &[("Log", 20)], workers: 0, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "Floodgate.IronTeeth", size: (1,1,3), sp: 150, unlocked: false, costs: &[("Log", 10), ("Plank", 5)], workers: 0, beds: 0, cap: None, fg: Some(1.0), p_out: 0.0, p_in: 0.0, build_h: 8.0),
    spec!(name: "DoubleFloodgate.IronTeeth", size: (1,1,2), sp: 250, unlocked: false, costs: &[("Log", 15), ("Plank", 10)], workers: 0, beds: 0, cap: None, fg: Some(2.0), p_out: 0.0, p_in: 0.0, build_h: 12.0),
    spec!(name: "Inventor.IronTeeth", size: (2,2,2), sp: 0, unlocked: true, costs: &[("Log", 12)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 0.0, build_h: 6.0),
    spec!(name: "LargePowerWheel.IronTeeth", size: (2,3,3), sp: 0, unlocked: true, costs: &[("Log", 25)], workers: 4, beds: 0, cap: None, fg: None, p_out: 300.0, p_in: 0.0, build_h: 10.0),
    spec!(name: "CompactWaterWheel.IronTeeth", size: (2,1,2), sp: 0, unlocked: true, costs: &[("Log", 40)], workers: 0, beds: 0, cap: None, fg: None, p_out: 120.0, p_in: 0.0, build_h: 10.0),
    spec!(name: "LumberMill.IronTeeth", size: (2,3,3), sp: 0, unlocked: true, costs: &[("Log", 15)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 50.0, build_h: 8.0),
    spec!(name: "GearWorkshop.IronTeeth", size: (2,3,2), sp: 100, unlocked: false, costs: &[("Log", 15), ("Plank", 25)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 120.0, build_h: 12.0),
    spec!(name: "WoodWorkshop.IronTeeth", size: (2,4,4), sp: 800, unlocked: false, costs: &[("Log", 20), ("Plank", 40), ("Gear", 40)], workers: 1, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 200.0, build_h: 18.0),
    spec!(name: "Smelter.IronTeeth", size: (2,4,3), sp: 300, unlocked: false, costs: &[("Plank", 50), ("Gear", 20), ("ScrapMetal", 30)], workers: 2, beds: 0, cap: None, fg: None, p_out: 0.0, p_in: 200.0, build_h: 18.0),
    spec!(name: "Engine.IronTeeth", size: (3,2,3), sp: 400, unlocked: false, costs: &[("Plank", 40), ("Gear", 30), ("MetalBlock", 20)], workers: 0, beds: 0, cap: None, fg: None, p_out: 400.0, p_in: 0.0, build_h: 16.0),
];

pub fn lookup_spec(name: &str) -> Option<&'static BuildingSpec> {
    BUILDING_SPECS.iter().find(|s| s.name == name)
}

pub fn build_work_hours_for(name: &str) -> f64 {
    lookup_spec(name)
        .map(|s| s.build_work_hours)
        .unwrap_or_else(|| {
            if name.contains("Path") || name.contains("Flag") {
                0.0
            } else {
                8.0
            }
        })
}

pub fn prefabs_for_faction(faction: &str) -> Vec<PrefabInfo> {
    let suffix = if faction.eq_ignore_ascii_case("IronTeeth") {
        ".IronTeeth"
    } else {
        ".Folktails"
    };
    BUILDING_SPECS
        .iter()
        .filter(|s| !s.name.contains('.') || s.name.ends_with(suffix))
        .map(|s| PrefabInfo {
            name: s.name.to_string(),
            size_x: s.size_x,
            size_y: s.size_y,
            size_z: s.size_z,
            science_cost: s.science_cost,
            unlocked: s.unlocked_default,
            cost: s
                .costs
                .iter()
                .map(|(good, amount)| PrefabCost {
                    good: (*good).to_string(),
                    amount: *amount,
                })
                .collect(),
        })
        .collect()
}
