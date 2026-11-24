use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Read-only summary extracted from a `.timber` ZIP archive (`world.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveReport {
    pub path: String,
    pub game_version: Option<String>,
    pub timestamp: Option<String>,
    pub map_size: Option<SaveMapSize>,
    pub day_number: Option<u32>,
    pub cycle: Option<u32>,
    pub cycle_day: Option<u32>,
    pub is_hazardous: Option<bool>,
    pub entity_count: usize,
    pub template_counts: BTreeMap<String, usize>,
    pub stock_totals: BTreeMap<String, i64>,
    pub terrain_stats: Option<TerrainStats>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveMapSize {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerrainStats {
    pub tile_count: usize,
    pub min_height: i32,
    pub max_height: i32,
    pub wet_tiles: usize,
    pub contaminated_tiles: usize,
}
