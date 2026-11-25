//! Unit and contract tests for 2D water/irrigation/floodgate grid, path connectivity,
//! hauling bottlenecks, and connected-component power network topology.

use timberbot_mock::data::*;
use timberbot_mock::sim::{HazardKind, MockWorld, ScenarioBuilding, ScenarioConfig};
use timberbot_protocol::http::{Orientation, PlaceBuildingRequest, Plant, TilesQuery};

#[test]
fn irrigation_2d_grid_respects_radius_and_badwater_cutoff() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.initial_crops = 0;
    cfg.initial_trees = 0;
    cfg.resources.insert("Water".to_string(), 500);
    cfg.resources.insert("Carrot".to_string(), 500);
    let mut world = MockWorld::from_scenario(cfg);

    // Tile (16, 14) is 5 blocks from river edge x=11 -> irrigated (<= 16)
    // Tile (30, 14) is 19 blocks from river edge x=11 -> dry (> 16)
    assert!(world.is_tile_irrigated(16, 14));
    assert!(!world.is_tile_irrigated(30, 14));

    let near_id = world.alloc_id();
    let far_id = world.alloc_id();
    world.crops.push(Plant {
        id: near_id,
        name: "Carrot".to_string(),
        x: 16,
        y: 14,
        z: 2,
        alive: 1,
        growth: Some(0.0),
        grown: Some(0),
        marked: None,
    });
    world.crops.push(Plant {
        id: far_id,
        name: "Carrot".to_string(),
        x: 30,
        y: 14,
        z: 2,
        alive: 1,
        growth: Some(0.0),
        grown: Some(0),
        marked: None,
    });

    world.step(2.0);
    let near = world.crops.iter().find(|c| c.id == near_id).unwrap();
    let far = world.crops.iter().find(|c| c.id == far_id).unwrap();
    assert!(
        near.growth.unwrap() >= 0.45,
        "irrigated crop near river should grow, got {:?}",
        near.growth
    );
    assert_eq!(
        far.growth.unwrap(),
        0.0,
        "crop beyond 16-tile 2D irrigation radius must NOT grow"
    );

    // Expose river to badwater without floodgates -> 2D moisture collapses
    world.is_hazardous = true;
    world.hazard_kind = HazardKind::Badtide;
    for b in &mut world.buildings {
        if b.floodgate == Some(1) {
            b.height = Some(0.0);
        }
    }
    world.step(1.5);
    let tiles = world
        .tiles(&TilesQuery {
            x1: Some(15),
            y1: Some(14),
            x2: Some(16),
            y2: Some(14),
        })
        .tiles
        .unwrap();
    assert!(
        tiles.iter().all(|t| t.moist == 0),
        "contaminated river must dry out 2D riverbank moisture"
    );
}

#[test]
fn off_center_floodgate_allows_2d_bypass_while_center_sluice_blocks() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.is_hazardous = true;
    cfg.hazard_kind = HazardKind::Badtide;
    cfg.river_contamination = 0.0;
    cfg.reservoir_water_depth = 1.5;
    cfg.buildings.retain(|b| !b.name.contains("Floodgate"));
    // Place an off-center floodgate only at x=9 (leaving x=10, 11 open!)
    cfg.buildings.push(ScenarioBuilding {
        name: "Floodgate.Folktails".to_string(),
        x: 9,
        y: 12,
        z: 1,
        orientation: Orientation::South,
        finished: true,
        paused: false,
        workers: None,
        floodgate_height: Some(1.0),
        max_height: Some(1.0),
        recipe: None,
        allowed_good: None,
        storage_mode: None,
    });
    let mut world = MockWorld::from_scenario(cfg);
    world.step(1.0);
    assert!(
        world.contam_grid[14 * 32 + 10] >= 0.85,
        "badwater must bypass an off-center floodgate through x=10"
    );
    assert!(
        world.contam_grid[14 * 32 + 9] > 0.3,
        "lateral 2D mixing must carry bypassed badwater back into column x=9 downstream"
    );
}

#[test]
fn path_connectivity_blocks_isolated_sites_until_linked() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.resources.insert("Log".to_string(), 80);
    let mut world = MockWorld::from_scenario(cfg);

    // Place a SmallTank far outside the DC core at (28, 28) -> unreachable!
    let placed = world
        .place_building(&PlaceBuildingRequest {
            prefab: "SmallTank.Folktails".to_string(),
            x: 28,
            y: 28,
            z: 2,
            orientation: Orientation::South,
        })
        .expect("place remote tank");
    let tank_id = placed["id"].as_i64().unwrap();
    let tank = world.buildings.iter().find(|b| b.id == tank_id).unwrap();
    assert_eq!(tank.reachable, Some(0), "remote site must be unreachable");

    // Step 2 full days -> builders cannot reach or build an unreachable site
    world.step(2.0);
    let tank = world.buildings.iter().find(|b| b.id == tank_id).unwrap();
    assert_eq!(tank.finished, 0);
    assert_eq!(tank.material_progress, Some(0.0));

    // Build a Path chain from DC core (20, 16) -> (28, 16) -> (28, 27)
    for x in (20..=28).step_by(2) {
        let _ = world.place_building(&PlaceBuildingRequest {
            prefab: "Path".to_string(),
            x,
            y: 16,
            z: 2,
            orientation: Orientation::South,
        });
    }
    for y in (18..=27).step_by(2) {
        let _ = world.place_building(&PlaceBuildingRequest {
            prefab: "Path".to_string(),
            x: 28,
            y,
            z: 2,
            orientation: Orientation::South,
        });
    }

    let tank = world.buildings.iter().find(|b| b.id == tank_id).unwrap();
    assert_eq!(
        tank.reachable,
        Some(1),
        "connecting Path chain must flip remote site to reachable=1"
    );

    world.step(2.0);
    let tank = world.buildings.iter().find(|b| b.id == tank_id).unwrap();
    assert_eq!(
        tank.finished, 1,
        "once linked by Path, builders construct the remote site"
    );
}

#[test]
fn disconnected_power_networks_require_power_shaft_bridge() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.resources.insert("Log".to_string(), 100);
    cfg.resources.insert("Plank".to_string(), 0);
    cfg.buildings = vec![
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
        // PowerWheel on west side of DC (12, 16)
        ScenarioBuilding {
            name: "PowerWheel.Folktails".to_string(),
            x: 12,
            y: 16,
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
        // LumberMill on east side of DC (20, 16) — Chebyshev distance 8 > 3 -> DISCONNECTED!
        ScenarioBuilding {
            name: "LumberMill.Folktails".to_string(),
            x: 20,
            y: 16,
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
    ];
    let mut world = MockWorld::from_scenario(cfg);

    // Two distinct disconnected PowerNetworks exist!
    let nets = world.power_networks();
    assert_eq!(
        nets.len(),
        2,
        "disconnected generator & mill must form 2 separate power networks"
    );
    let mill = world
        .buildings
        .iter()
        .find(|b| b.name.contains("LumberMill"))
        .unwrap();
    assert_eq!(
        mill.powered,
        Some(0),
        "disconnected LumberMill must stay unpowered despite 50 hp surplus in other network"
    );
    world.step(1.0);
    assert_eq!(
        *world.resources.get("Plank").unwrap_or(&0.0),
        0.0,
        "disconnected LumberMill produces 0 planks"
    );

    // Bridge the gap with PowerShafts at (15, 16) and (18, 16)
    for x in [15, 18] {
        world
            .place_building(&PlaceBuildingRequest {
                prefab: "PowerShaft.Folktails".to_string(),
                x,
                y: 16,
                z: 2,
                orientation: Orientation::South,
            })
            .expect("place power shaft");
    }
    // Step 1 day to finish shafts + 1 day to mill planks
    world.step(1.0);
    let nets_after = world.power_networks();
    assert_eq!(
        nets_after.len(),
        1,
        "PowerShafts must merge the two networks into 1 connected power network"
    );
    world.step(1.0);
    let mill = world
        .buildings
        .iter()
        .find(|b| b.name.contains("LumberMill"))
        .unwrap();
    assert_eq!(mill.powered, Some(1));
    assert!(
        *world.resources.get("Plank").unwrap_or(&0.0) >= 8.0,
        "bridged LumberMill must now produce Planks"
    );
}

#[test]
fn hauling_bottleneck_penalizes_zero_hauler_colony() {
    let make_world = |adults: u32| {
        let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
        cfg.adults = adults;
        cfg.children = 0;
        cfg.resources.insert("Water".to_string(), 0);
        cfg.resources.insert("Carrot".to_string(), 500);
        cfg.buildings = vec![
            ScenarioBuilding {
                name: "DistrictCenter.Folktails".to_string(),
                x: 16,
                y: 16,
                z: 2,
                orientation: Orientation::South,
                finished: true,
                paused: false,
                workers: Some(0), // 0 builders
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
                name: "Inventor.Folktails".to_string(),
                x: 15,
                y: 15,
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
                name: "LumberjackFlag.Folktails".to_string(),
                x: 16,
                y: 18,
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
                x: 14,
                y: 18,
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
        ];
        MockWorld::from_scenario(cfg)
    };

    // 4 adults across 4 workplaces -> 0 idle haulers -> hauling bottleneck (0.75x)
    let bottlenecked = make_world(4);
    assert_eq!(bottlenecked.hauling_efficiency(), HAULING_BOTTLENECK_MULT);

    // 6 adults across 4 workplaces -> 2 idle haulers -> full efficiency (1.0x)
    let healthy = make_world(6);
    assert_eq!(healthy.hauling_efficiency(), 1.0);
}
