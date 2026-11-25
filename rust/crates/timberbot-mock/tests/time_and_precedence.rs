//! Unit and contract tests for time, construction queues, material/tech/power/irrigation
//! precedence chains, walking/hauling distance efficiency, and 1D water propagation lag.

use timberbot_mock::data::*;
use timberbot_mock::sim::{HazardKind, MockWorld, ScenarioBuilding, ScenarioConfig};
use timberbot_protocol::http::{Orientation, PlaceBuildingRequest, Priority, TilesQuery};

#[test]
fn builder_queue_non_instant_work_hours_priority_and_zero_builder_stall() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.resources.insert("Log".to_string(), 100);
    cfg.resources.insert("Plank".to_string(), 40);
    // Start with 0 builders on DistrictCenter
    for b in &mut cfg.buildings {
        if b.name.starts_with("DistrictCenter") {
            b.workers = Some(0);
            b.paused = true;
        }
    }
    let mut world = MockWorld::from_scenario(cfg);

    // 1. Placing a Dam does NOT finish instantly
    let placed = world
        .place_building(&PlaceBuildingRequest {
            prefab: "Dam.Folktails".to_string(),
            x: 10,
            y: 18,
            z: 1,
            orientation: Orientation::South,
        })
        .expect("place dam");
    let dam_id = placed["id"].as_i64().unwrap();
    let dam = world.buildings.iter().find(|b| b.id == dam_id).unwrap();
    assert_eq!(dam.finished, 0, "Dam must not appear instantly");
    assert_eq!(dam.material_progress.unwrap_or(1.0), 0.0);
    assert_eq!(dam.build_progress.unwrap_or(1.0), 0.0);

    // 2. With 0 active builders, stepping 2 full days advances neither hauling nor building
    world.step(2.0);
    let dam = world.buildings.iter().find(|b| b.id == dam_id).unwrap();
    assert_eq!(dam.finished, 0);
    assert_eq!(dam.material_progress.unwrap_or(1.0), 0.0);
    assert_eq!(dam.build_progress.unwrap_or(1.0), 0.0);

    // 3. Re-enable 1 builder on DistrictCenter and queue a VeryLow Lodge + VeryHigh SmallTank
    let dc = world
        .buildings
        .iter_mut()
        .find(|b| b.name.starts_with("DistrictCenter"))
        .unwrap();
    dc.paused = 0;
    dc.desired_workers = Some(1);
    world.rebalance_workers_and_housing();

    // Pause the Dam so we strictly test Lodge (VeryLow) vs SmallTank (VeryHigh) queue ordering
    world
        .buildings
        .iter_mut()
        .find(|b| b.id == dam_id)
        .unwrap()
        .paused = 1;

    let lodge_id = world
        .place_building(&PlaceBuildingRequest {
            prefab: "Lodge.Folktails".to_string(),
            x: 18,
            y: 16,
            z: 2,
            orientation: Orientation::South,
        })
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let tank_id = world
        .place_building(&PlaceBuildingRequest {
            prefab: "SmallTank.Folktails".to_string(),
            x: 17,
            y: 16,
            z: 2,
            orientation: Orientation::South,
        })
        .unwrap()["id"]
        .as_i64()
        .unwrap();

    world
        .buildings
        .iter_mut()
        .find(|b| b.id == lodge_id)
        .unwrap()
        .construction_priority = Some(Priority::VeryLow);
    world
        .buildings
        .iter_mut()
        .find(|b| b.id == tank_id)
        .unwrap()
        .construction_priority = Some(Priority::VeryHigh);

    // Step 0.06 days (0.96 builder-hours): should haul materials to SmallTank first, Lodge still at 0
    world.step(0.06);
    let tank = world.buildings.iter().find(|b| b.id == tank_id).unwrap();
    let lodge = world.buildings.iter().find(|b| b.id == lodge_id).unwrap();
    assert!(
        tank.material_progress.unwrap_or(0.0) > 0.5 && tank.material_progress.unwrap_or(0.0) < 1.0,
        "SmallTank should be mid-haul after 0.96 builder-hours, got {:?}",
        tank.material_progress
    );
    assert_eq!(
        tank.build_progress.unwrap_or(1.0),
        0.0,
        "build_progress must not start before hauling finishes"
    );
    assert_eq!(
        lodge.material_progress.unwrap_or(1.0),
        0.0,
        "VeryLow Lodge must wait in queue behind VeryHigh SmallTank"
    );

    // Step another 0.35 days (5.6 builder-hours): SmallTank finishes (1.25h haul + 4.0h build), Lodge starts
    world.step(0.35);
    let tank = world.buildings.iter().find(|b| b.id == tank_id).unwrap();
    let lodge = world.buildings.iter().find(|b| b.id == lodge_id).unwrap();
    assert_eq!(tank.finished, 1, "VeryHigh SmallTank must finish first");
    assert_eq!(
        lodge.finished, 0,
        "VeryLow Lodge should still be in progress"
    );
    assert!(
        lodge.material_progress.unwrap_or(0.0) > 0.0,
        "Builder queue must move on to Lodge once SmallTank finishes"
    );
}

#[test]
fn full_material_power_and_science_precedence_chain() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.science = 50; // Too low for GearWorkshop (100 SP), Smelter (300 SP), WoodWorkshop (800 SP)
    cfg.resources.insert("Log".to_string(), 120);
    cfg.resources.insert("Plank".to_string(), 0);
    cfg.resources.insert("Gear".to_string(), 0);
    cfg.resources.insert("PineResin".to_string(), 30);
    cfg.resources.insert("ScrapMetal".to_string(), 60);
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
        ScenarioBuilding {
            name: "LumberMill.Folktails".to_string(),
            x: 17,
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

    // 1. Locked building placement is rejected until science unlock
    let err = world
        .place_building(&PlaceBuildingRequest {
            prefab: "GearWorkshop.Folktails".to_string(),
            x: 18,
            y: 16,
            z: 2,
            orientation: Orientation::South,
        })
        .expect_err("GearWorkshop must be rejected while locked");
    assert!(err.contains("locked"), "expected locked error, got {err}");

    // 2. Unpowered LumberMill produces 0 planks even with logs + worker
    world.step(1.0);
    assert_eq!(
        *world.resources.get("Plank").unwrap_or(&0.0),
        0.0,
        "unpowered LumberMill must produce 0 planks"
    );

    // 3. Add a finished PowerWheel (50 hp == LumberMill 50 hp demand) -> LumberMill produces Planks
    let pw = world
        .place_building(&PlaceBuildingRequest {
            prefab: "PowerWheel.Folktails".to_string(),
            x: 16,
            y: 17,
            z: 2,
            orientation: Orientation::South,
        })
        .expect("place power wheel");
    let pw_id = pw["id"].as_i64().unwrap();
    // Let builders finish the PowerWheel (~10h work -> 0.5 day with 2 builders)
    world.step(1.0);
    assert_eq!(
        world
            .buildings
            .iter()
            .find(|b| b.id == pw_id)
            .unwrap()
            .finished,
        1
    );
    // Step 2 more days to stockpile Planks from Logs
    world.step(2.0);
    let planks = *world.resources.get("Plank").unwrap_or(&0.0);
    assert!(
        planks >= 20.0,
        "powered LumberMill should convert Logs -> Planks, got {planks}"
    );

    // 4. Unlock and build GearWorkshop (100 SP), WoodWorkshop (800 SP), Smelter (300 SP)
    world.science = 1500;
    world
        .unlock_building("GearWorkshop.Folktails")
        .expect("unlock gear");
    world
        .unlock_building("WoodWorkshop.Folktails")
        .expect("unlock wood");
    world
        .unlock_building("Smelter.Folktails")
        .expect("unlock smelter");
    // Ensure enough materials & power (2 WaterWheels = 360 hp + 1 PowerWheel = 50 hp -> 410 hp;
    // plus extra WaterWheel = 590 hp >= 50 + 120 + 200 + 200 = 570 hp)
    world.resources.insert("Plank".to_string(), 160.0);
    world.resources.insert("Gear".to_string(), 60.0);
    world.resources.insert("Log".to_string(), 250.0);

    for (prefab, x, y, z) in [
        ("GearWorkshop.Folktails", 18, 16, 2),
        ("WoodWorkshop.Folktails", 19, 16, 2),
        ("Smelter.Folktails", 20, 16, 2),
        ("WaterWheel.Folktails", 10, 16, 1),
        ("WaterWheel.Folktails", 10, 17, 1),
        ("WaterWheel.Folktails", 10, 18, 1),
    ] {
        world
            .place_building(&PlaceBuildingRequest {
                prefab: prefab.to_string(),
                x,
                y,
                z,
                orientation: Orientation::South,
            })
            .unwrap_or_else(|e| panic!("place {prefab}: {e}"));
    }
    // Wait for builders to construct all buildings (non-instant!)
    world.step(6.0);
    let gears_before = *world.resources.get("Gear").unwrap_or(&0.0);
    let treated_before = *world.resources.get("TreatedPlank").unwrap_or(&0.0);
    let metal_before = *world.resources.get("MetalBlock").unwrap_or(&0.0);
    world.step(2.0);
    assert!(
        *world.resources.get("Gear").unwrap_or(&0.0) > gears_before,
        "powered GearWorkshop must convert Planks -> Gears"
    );
    assert!(
        *world.resources.get("TreatedPlank").unwrap_or(&0.0) > treated_before,
        "powered WoodWorkshop must convert Planks + PineResin -> TreatedPlanks"
    );
    assert!(
        *world.resources.get("MetalBlock").unwrap_or(&0.0) > metal_before,
        "powered Smelter must convert ScrapMetal -> MetalBlocks"
    );
}

#[test]
fn irrigation_prerequisite_for_crop_and_tree_growth_and_forester_planting() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.is_hazardous = true;
    cfg.hazard_kind = HazardKind::Drought;
    cfg.temperate_weather_duration = 50;
    cfg.hazardous_weather_duration = 50;
    cfg.reservoir_water_depth = 0.0; // Dry river -> no irrigation
    cfg.initial_trees = 0;
    cfg.initial_crops = 0;
    cfg.science = 50;
    cfg.resources.insert("Water".to_string(), 500);
    cfg.resources.insert("Carrot".to_string(), 500);
    cfg.resources.insert("Log".to_string(), 40);
    cfg.resources.insert("Plank".to_string(), 20);
    let mut world = MockWorld::from_scenario(cfg);

    // Add 1 sapling Pine (12d), 1 sapling Oak (30d), and 1 young Carrot (4d)
    let id1 = world.alloc_id();
    let id2 = world.alloc_id();
    let id3 = world.alloc_id();
    world.trees.push(timberbot_protocol::http::Plant {
        id: id1,
        name: "Pine".to_string(),
        x: 22,
        y: 12,
        z: 2,
        alive: 1,
        growth: Some(0.0),
        grown: Some(0),
        marked: Some(1),
    });
    world.trees.push(timberbot_protocol::http::Plant {
        id: id2,
        name: "Oak".to_string(),
        x: 23,
        y: 12,
        z: 2,
        alive: 1,
        growth: Some(0.0),
        grown: Some(0),
        marked: Some(1),
    });
    world.crops.push(timberbot_protocol::http::Plant {
        id: id3,
        name: "Carrot".to_string(),
        x: 18,
        y: 14,
        z: 2,
        alive: 1,
        growth: Some(0.0),
        grown: Some(0),
        marked: None,
    });

    // Dry river -> zero crop and tree growth over 3 days
    world.step(3.0);
    assert_eq!(world.trees[0].growth.unwrap(), 0.0, "dry Pine cannot grow");
    assert_eq!(
        world.crops[0].growth.unwrap(),
        0.0,
        "dry Carrot cannot grow"
    );

    // Restore temperate irrigated water
    world.is_hazardous = false;
    world.cycle_day = 1;
    world.river_depth.fill(1.5);
    world.reservoir_water_depth = 1.5;

    // Unlock and place Forester (30 SP)
    let locked_err = world
        .place_building(&PlaceBuildingRequest {
            prefab: "Forester.Folktails".to_string(),
            x: 21,
            y: 16,
            z: 2,
            orientation: Orientation::South,
        })
        .expect_err("Forester must require science unlock");
    assert!(locked_err.contains("locked"));
    world
        .unlock_building("Forester.Folktails")
        .expect("unlock forester");
    world
        .place_building(&PlaceBuildingRequest {
            prefab: "Forester.Folktails".to_string(),
            x: 21,
            y: 16,
            z: 2,
            orientation: Orientation::South,
        })
        .expect("place forester");

    // Step 12 days day-by-day so builders finish the Forester on day 1, a beaver staffs it,
    // and it plants new Pine saplings while the original Pine (12d) matures and Oak (30d) reaches 40%.
    for _ in 0..12 {
        world.step(1.0);
    }
    assert_eq!(world.trees[0].grown, Some(1), "Pine matures in 12 days");
    assert_eq!(world.trees[1].grown, Some(0), "Oak takes 30 days, not 12");
    assert!(
        (world.trees[1].growth.unwrap() - 0.4).abs() < 0.05,
        "Oak at day 12 should be ~40% grown, got {:?}",
        world.trees[1].growth
    );
    assert!(
        world.trees.len() > 2,
        "staffed Forester must plant new Pine saplings when irrigated"
    );
}

#[test]
fn walking_and_hauling_distance_reduces_efficiency() {
    assert_eq!(distance_efficiency(4), 1.0);
    assert!(distance_efficiency(20) < 0.8);

    // Compare two identical worlds with a WaterPump near DC (dist 4) vs far from DC (dist 22)
    let make_world = |px: i32, py: i32| {
        let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
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
                workers: Some(1),
                floodgate_height: None,
                max_height: None,
                recipe: None,
                allowed_good: None,
                storage_mode: None,
            },
            ScenarioBuilding {
                name: "WaterPump.Folktails".to_string(),
                x: px,
                y: py,
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

    let mut near = make_world(12, 16); // dist 4 -> eff 1.0
    let mut far = make_world(12, 30); // dist 18 -> eff 1 / (1 + 12*0.025) = 0.769
    near.step(1.0);
    far.step(1.0);
    let w_near = *near.resources.get("Water").unwrap_or(&0.0);
    let w_far = *far.resources.get("Water").unwrap_or(&0.0);
    assert!(
        w_near > w_far + 5.0,
        "distant workplace must suffer walking penalty: near={w_near}, far={w_far}"
    );
}

#[test]
fn water_propagation_lag_across_tiles_and_floodgate_diversion() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.is_hazardous = true;
    cfg.hazard_kind = HazardKind::Badtide;
    cfg.river_contamination = 0.0;
    cfg.reservoir_water_depth = 1.5;
    for b in &mut cfg.buildings {
        if b.name.contains("Floodgate") {
            b.floodgate_height = Some(0.0);
            b.y = 12;
        }
    }
    let mut open = MockWorld::from_scenario(cfg.clone());

    // After 0.25 days (6 game-hours = 6 tiles at 24 tiles/day), upstream y=2 is contaminated
    // while downstream intake y=14 is still clean (real water propagation lag!)
    open.step(0.25);
    let tiles = open
        .tiles(&TilesQuery {
            x1: Some(10),
            y1: Some(0),
            x2: Some(10),
            y2: Some(20),
        })
        .tiles
        .unwrap();
    let up = tiles.iter().find(|t| t.y == 2).unwrap();
    let intake = tiles.iter().find(|t| t.y == 14).unwrap();
    assert!(
        up.badwater > 0.5,
        "upstream y=2 must already be contaminated after 6h, got {}",
        up.badwater
    );
    assert!(
        intake.badwater < 0.05,
        "downstream intake y=14 must still be clean after 6h due to propagation lag, got {}",
        intake.badwater
    );

    // After another 0.75 days (total 1.0d = 24 tiles), unblocked badwater reaches intake y=14
    open.step(0.75);
    assert!(
        open.river_contamination >= 0.9,
        "unblocked badwater must reach intake y=14 by 1.0d, got {}",
        open.river_contamination
    );

    // With Floodgate at y=12 raised to 1.0, upstream y=6 is contaminated while intake y=14 stays clean
    for b in &mut cfg.buildings {
        if b.name.contains("Floodgate") {
            b.floodgate_height = Some(1.0);
        }
    }
    let mut gated = MockWorld::from_scenario(cfg);
    gated.step(1.0);
    assert!(
        gated.river_contam[6] > 0.8,
        "upstream of raised floodgate (y=6) must carry badwater"
    );
    assert!(
        gated.river_contam[14] < 0.05,
        "downstream of raised floodgate (y=14) must remain clean"
    );
}
