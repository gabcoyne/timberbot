use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde_json::Value;
use thiserror::Error;
use timberbot_protocol::save::{SaveMapSize, SaveReport, TerrainStats};
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("missing world.json inside .timber archive")]
    MissingWorldJson,

    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SaveError>;

/// Read-only inspection of a Timberborn `.timber` ZIP save archive.
/// Never writes or modifies the save file.
pub fn inspect_save(path: impl AsRef<Path>) -> Result<SaveReport> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref)?;
    let mut archive = ZipArchive::new(file)?;

    let mut world_entry = archive
        .by_name("world.json")
        .map_err(|_| SaveError::MissingWorldJson)?;
    let mut contents = String::new();
    world_entry.read_to_string(&mut contents)?;

    let root: Value = serde_json::from_str(&contents)?;
    Ok(analyze_world_json(&path_ref.display().to_string(), &root))
}

/// Analyze a parsed `world.json` document (also usable in unit tests without disk I/O).
pub fn analyze_world_json(path_label: &str, root: &Value) -> SaveReport {
    let mut warnings = Vec::new();

    let game_version = root
        .get("GameVersion")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    if game_version.is_none() {
        warnings.push("GameVersion field missing in world.json".to_string());
    }

    let timestamp = root
        .get("Timestamp")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    let singletons = root.get("Singletons");

    let map_size = singletons
        .and_then(|s| s.get("MapSize"))
        .and_then(|m| m.get("Size"))
        .and_then(|sz| {
            let x = sz.get("X")?.as_u64()? as u32;
            let y = sz.get("Y")?.as_u64()? as u32;
            Some(SaveMapSize { x, y })
        });

    let day_number = singletons
        .and_then(|s| s.get("DayNightCycle"))
        .and_then(|d| d.get("DayNumber"))
        .and_then(Value::as_u64)
        .map(|v| v as u32);

    let weather_singleton = singletons.and_then(|s| {
        s.get("WeatherService")
            .or_else(|| s.get("HazardousWeatherService"))
            .or_else(|| s.get("GameCycleService"))
    });

    let cycle = singletons
        .and_then(|s| s.get("GameCycleService").or(weather_singleton))
        .and_then(|g| g.get("Cycle"))
        .and_then(Value::as_u64)
        .map(|v| v as u32);

    let cycle_day = singletons
        .and_then(|s| s.get("GameCycleService").or(weather_singleton))
        .and_then(|g| g.get("CycleDay"))
        .and_then(Value::as_u64)
        .map(|v| v as u32);

    let is_hazardous = singletons
        .and_then(|s| {
            s.get("WeatherService")
                .or_else(|| s.get("HazardousWeatherService"))
        })
        .and_then(|w| {
            w.get("IsHazardousWeather")
                .or_else(|| w.get("IsHazardous"))
                .or_else(|| w.get("InDrought"))
        })
        .and_then(Value::as_bool);

    let terrain_stats = extract_terrain_stats(singletons);

    let mut template_counts = BTreeMap::new();
    let mut stock_totals = BTreeMap::new();
    let mut entity_count = 0usize;

    if let Some(entities) = root.get("Entities").and_then(Value::as_array) {
        entity_count = entities.len();
        for ent in entities {
            if let Some(tpl) = ent.get("Template").and_then(Value::as_str) {
                *template_counts.entry(tpl.to_string()).or_insert(0) += 1;
            }
            if let Some(comps) = ent.get("Components").and_then(Value::as_object) {
                for (comp_name, comp_val) in comps {
                    if comp_name.contains("Inventory") {
                        accumulate_inventory(comp_val, &mut stock_totals);
                    }
                }
            }
        }
    } else {
        warnings.push("Entities array missing or non-array in world.json".to_string());
    }

    SaveReport {
        path: path_label.to_string(),
        game_version,
        timestamp,
        map_size,
        day_number,
        cycle,
        cycle_day,
        is_hazardous,
        entity_count,
        template_counts,
        stock_totals,
        terrain_stats,
        warnings,
    }
}

fn extract_terrain_stats(singletons: Option<&Value>) -> Option<TerrainStats> {
    let singletons = singletons?;
    let heights_str = singletons
        .get("TerrainMap")
        .and_then(|t| t.get("Heights"))
        .and_then(|h| h.get("Array"))
        .and_then(Value::as_str)?;

    let heights: Vec<i32> = heights_str
        .split_whitespace()
        .filter_map(|tok| tok.parse::<i32>().ok())
        .collect();
    if heights.is_empty() {
        return None;
    }

    let min_height = *heights.iter().min().unwrap_or(&0);
    let max_height = *heights.iter().max().unwrap_or(&0);

    let wet_tiles = singletons
        .get("WaterMap")
        .or_else(|| singletons.get("WaterMapNew"))
        .and_then(|w| w.get("WaterDepths").or_else(|| w.get("WaterColumns")))
        .and_then(|d| d.get("Array"))
        .and_then(Value::as_str)
        .map(|s| {
            s.split_whitespace()
                .filter_map(|tok| tok.parse::<f64>().ok())
                .filter(|&v| v > 0.01)
                .count()
        })
        .unwrap_or(0);

    let contaminated_tiles = singletons
        .get("WaterMap")
        .or_else(|| singletons.get("WaterMapNew"))
        .and_then(|w| w.get("Contaminations").or_else(|| w.get("Outflows")))
        .and_then(|d| d.get("Array"))
        .and_then(Value::as_str)
        .map(|s| {
            s.split_whitespace()
                .filter_map(|tok| tok.parse::<f64>().ok())
                .filter(|&v| v > 0.01)
                .count()
        })
        .unwrap_or(0);

    Some(TerrainStats {
        tile_count: heights.len(),
        min_height,
        max_height,
        wet_tiles,
        contaminated_tiles,
    })
}

fn accumulate_inventory(comp: &Value, totals: &mut BTreeMap<String, i64>) {
    let Some(storage) = comp
        .get("Storage")
        .and_then(|s| s.get("Goods"))
        .or_else(|| comp.get("Goods"))
        .and_then(Value::as_array)
    else {
        return;
    };

    for item in storage {
        let good = item
            .get("Good")
            .and_then(|g| g.get("Id"))
            .or_else(|| item.get("Good"))
            .and_then(Value::as_str);
        let amount = item.get("Amount").and_then(Value::as_i64);
        if let (Some(g), Some(a)) = (good, amount) {
            *totals.entry(g.to_string()).or_insert(0) += a;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn analyzes_world_json_singletons_and_entities() {
        let doc = json!({
            "GameVersion": "1.1.2.0",
            "Timestamp": "2026-11-24 12:00:00",
            "Singletons": {
                "MapSize": { "Size": { "X": 64, "Y": 64 } },
                "DayNightCycle": { "DayNumber": 12 },
                "GameCycleService": { "Cycle": 2, "CycleDay": 5 },
                "WeatherService": { "IsHazardousWeather": true },
                "TerrainMap": { "Heights": { "Array": "1 2 2 3" } },
                "WaterMap": {
                    "WaterDepths": { "Array": "0.0 1.2 0.8 0.0" },
                    "Contaminations": { "Array": "0.0 0.0 0.5 0.0" }
                }
            },
            "Entities": [
                {
                    "Template": "DistrictCenter.Folktails",
                    "Components": {
                        "Inventory": {
                            "Storage": {
                                "Goods": [
                                    { "Good": { "Id": "Water" }, "Amount": 42 },
                                    { "Good": { "Id": "Log" }, "Amount": 19 }
                                ]
                            }
                        }
                    }
                },
                { "Template": "Lodge.Folktails", "Components": {} }
            ]
        });

        let report = analyze_world_json("test.timber", &doc);
        assert_eq!(report.game_version.as_deref(), Some("1.1.2.0"));
        assert_eq!(report.day_number, Some(12));
        assert_eq!(report.cycle, Some(2));
        assert_eq!(report.is_hazardous, Some(true));
        assert_eq!(report.entity_count, 2);
        assert_eq!(report.stock_totals.get("Water"), Some(&42));
        assert_eq!(report.stock_totals.get("Log"), Some(&19));
        let ts = report.terrain_stats.expect("terrain stats");
        assert_eq!(ts.tile_count, 4);
        assert_eq!(ts.wet_tiles, 2);
        assert_eq!(ts.contaminated_tiles, 1);
    }
}
