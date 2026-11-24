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

/// Read-only inspection of a Timberborn `.timber` ZIP save or map archive.
/// Never writes or modifies the archive.
pub fn inspect_save(path: impl AsRef<Path>) -> Result<SaveReport> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref)?;
    let mut archive = ZipArchive::new(file)?;

    let version_txt = archive.by_name("version.txt").ok().and_then(|mut entry| {
        let mut s = String::new();
        entry.read_to_string(&mut s).ok()?;
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let mut world_entry = archive
        .by_name("world.json")
        .map_err(|_| SaveError::MissingWorldJson)?;
    let mut contents = String::new();
    world_entry.read_to_string(&mut contents)?;

    let root: Value = serde_json::from_str(&contents)?;
    let mut report = analyze_world_json(&path_ref.display().to_string(), &root);
    if report.game_version.is_none() {
        report.game_version = version_txt;
    }
    Ok(report)
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

    let terrain_stats = extract_terrain_stats(singletons, map_size.as_ref());

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
                    if comp_name.contains("Inventory") && comp_name != "Inventory:ConstructionSite"
                    {
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

fn extract_terrain_stats(
    singletons: Option<&Value>,
    map_size: Option<&SaveMapSize>,
) -> Option<TerrainStats> {
    let singletons = singletons?;
    let terrain = singletons.get("TerrainMap")?;

    let heights: Vec<i32> = if let Some(heights_str) = terrain
        .get("Heights")
        .and_then(|h| h.get("Array"))
        .and_then(Value::as_str)
    {
        heights_str
            .split_whitespace()
            .filter_map(|tok| tok.parse::<i32>().ok())
            .collect()
    } else {
        let voxels_str = terrain
            .get("Voxels")
            .and_then(|v| v.get("Array"))
            .and_then(Value::as_str)?;
        // Timberborn 1.0/1.1 stores TerrainMap.Voxels.Array as 23 vertical layers
        // indexed `z * X * Y + y * X + x` with 0/1 solid voxels (verified via
        // 0xguy07/timberborn-tools on 1.0.13.1 & 1.1.2.4).
        let (x, y) = map_size.map(|m| (m.x as usize, m.y as usize))?;
        let layer_size = x.saturating_mul(y);
        if layer_size == 0 {
            return None;
        }
        let mut col_heights = vec![0i32; layer_size];
        for (idx, tok) in voxels_str.split_whitespace().enumerate() {
            if tok == "1" {
                let z = (idx / layer_size) as i32;
                let cell = idx % layer_size;
                if z + 1 > col_heights[cell] {
                    col_heights[cell] = z + 1;
                }
            }
        }
        col_heights
    };

    if heights.is_empty() {
        return None;
    }

    let min_height = *heights.iter().min().unwrap_or(&0);
    let max_height = *heights.iter().max().unwrap_or(&0);

    let (wet_tiles, contaminated_tiles) = extract_water_counts(singletons);

    Some(TerrainStats {
        tile_count: heights.len(),
        min_height,
        max_height,
        wet_tiles,
        contaminated_tiles,
    })
}

fn extract_water_counts(singletons: &Value) -> (usize, usize) {
    let mut wet = 0usize;
    let mut contaminated = 0usize;

    if let Some(cols_str) = singletons
        .get("WaterMapNew")
        .and_then(|w| w.get("WaterColumns"))
        .and_then(|d| d.get("Array"))
        .and_then(Value::as_str)
    {
        let mut saw_colon_format = false;
        for tok in cols_str.split_whitespace() {
            if tok == "0" {
                continue;
            }
            if tok.contains(':') {
                saw_colon_format = true;
                // 1.0/1.1 column format: "depth:contamination:?:floorZ:prev"
                let mut parts = tok.split(':');
                let depth = parts
                    .next()
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let contam = parts
                    .next()
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(0.0);
                if depth > 0.01 {
                    wet += 1;
                }
                if contam > 0.01 {
                    contaminated += 1;
                }
            } else if let Ok(depth) = tok.parse::<f64>() {
                if depth > 0.01 {
                    wet += 1;
                }
            }
        }
        if !saw_colon_format {
            contaminated = singletons
                .get("WaterMapNew")
                .and_then(|w| w.get("Contamination").or_else(|| w.get("Contaminations")))
                .and_then(|d| d.get("Array"))
                .and_then(Value::as_str)
                .map(|s| {
                    s.split_whitespace()
                        .filter_map(|tok| tok.parse::<f64>().ok())
                        .filter(|&v| v > 0.01)
                        .count()
                })
                .unwrap_or(0);
        }
        return (wet, contaminated);
    }

    if let Some(wm) = singletons.get("WaterMap") {
        wet = wm
            .get("WaterDepths")
            .and_then(|d| d.get("Array"))
            .and_then(Value::as_str)
            .map(|s| {
                s.split_whitespace()
                    .filter_map(|tok| tok.parse::<f64>().ok())
                    .filter(|&v| v > 0.01)
                    .count()
            })
            .unwrap_or(0);
        contaminated = wm
            .get("Contaminations")
            .and_then(|d| d.get("Array"))
            .and_then(Value::as_str)
            .map(|s| {
                s.split_whitespace()
                    .filter_map(|tok| tok.parse::<f64>().ok())
                    .filter(|&v| v > 0.01)
                    .count()
            })
            .unwrap_or(0);
    }

    (wet, contaminated)
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
    use std::path::PathBuf;

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

    #[test]
    fn analyzes_timberborn_1_1_voxels_and_colon_water_columns() {
        // 2x2 map with 3 layers (z=0 all solid, z=1 cells 0,1 solid, z=2 cell 0 solid -> heights [3,2,1,1])
        let doc = json!({
            "GameVersion": "1.1.2.4",
            "Timestamp": "2026-10-15 09:30:00",
            "Singletons": {
                "MapSize": { "Size": { "X": 2, "Y": 2 } },
                "DayNightCycle": { "DayNumber": 48 },
                "GameCycleService": { "Cycle": 4, "CycleDay": 15 },
                "WeatherService": { "IsHazardousWeather": true },
                "TerrainMap": { "Voxels": { "Array": "1 1 1 1 1 1 0 0 1 0 0 0" } },
                "WaterMapNew": {
                    "Levels": 1,
                    "WaterColumns": { "Array": "0 1.4:0.0:0:2:0 1.1:0.75:0:1:0 0" }
                }
            },
            "Entities": [
                {
                    "Template": "SmallTank.Folktails",
                    "Components": {
                        "Inventory:Good": {
                            "Storage": { "Goods": [{ "Good": "Water", "Amount": 30 }] }
                        },
                        "Inventory:ConstructionSite": {
                            "Storage": { "Goods": [{ "Good": "Log", "Amount": 15 }] }
                        }
                    }
                }
            ]
        });

        let report = analyze_world_json("real_1_1_2_4.timber", &doc);
        assert_eq!(report.game_version.as_deref(), Some("1.1.2.4"));
        let ts = report.terrain_stats.expect("voxel terrain stats");
        assert_eq!(ts.tile_count, 4);
        assert_eq!(ts.min_height, 1);
        assert_eq!(ts.max_height, 3);
        assert_eq!(ts.wet_tiles, 2);
        assert_eq!(ts.contaminated_tiles, 1);
        assert_eq!(report.stock_totals.get("Water"), Some(&30));
        // Inventory:ConstructionSite is excluded from available colony stock
        assert_eq!(report.stock_totals.get("Log"), None);
    }

    #[test]
    fn inspects_real_practice_valley_timber_archive() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../scenarios/practice-valley.timber");
        let report = inspect_save(&path).expect("inspect practice-valley.timber");
        assert_eq!(report.game_version.as_deref(), Some("1.0.13.1-b769e88-sm"));
        assert_eq!(report.map_size, Some(SaveMapSize { x: 128, y: 128 }));
        assert_eq!(report.entity_count, 1644);
        assert_eq!(report.template_counts.get("WaterSource"), Some(&20));
        assert_eq!(report.template_counts.get("Pine"), Some(&520));
        let ts = report.terrain_stats.expect("voxel terrain stats");
        assert_eq!(ts.tile_count, 128 * 128);
        assert!(ts.max_height > ts.min_height);
    }
}
