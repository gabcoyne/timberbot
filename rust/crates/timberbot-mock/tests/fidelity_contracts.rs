use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::{json, Value};
use timberbot_mock::data::{
    Difficulty, BREEDING_POD_CYCLE_DAYS, EVAP_NARROW_CHANNEL_PER_DAY, EVAP_WIDE_POOL_PER_DAY,
    FOOD_PER_BEAVER_DAY, INVENTOR_SP_PER_HOUR, STARVATION_DEATH_DAYS, WATER_PER_BEAVER_DAY,
    WATER_PUMP_PER_HOUR,
};
use timberbot_mock::sim::{HazardKind, MockWorld, ScenarioBuilding, ScenarioConfig};
use timberbot_mock::{MockServerConfig, RunningMockServer};
use timberbot_protocol::http::Orientation;

#[test]
fn golden_difficulty_presets_and_consumption_rates() {
    // Verified wiki Game_Mode starting pop/food/water & 40% Easy consumption
    for (diff, exp_food, exp_water, cons_mult) in [
        (Difficulty::Easy, 300, 250, 0.40),
        (Difficulty::Normal, 130, 0, 1.00),
        (Difficulty::Hard, 90, 0, 1.00),
    ] {
        let cfg = ScenarioConfig::preset("Folktails", diff);
        assert_eq!(cfg.adults, 9);
        assert_eq!(cfg.children, 4);
        assert_eq!(cfg.resources.get("Berries"), Some(&exp_food));
        assert_eq!(cfg.resources.get("Water"), Some(&exp_water));

        // Pure consumption check (no buildings producing)
        let mut bare = cfg.clone();
        bare.buildings.clear();
        bare.resources.insert("Water".to_string(), 1000);
        bare.resources.insert("Carrot".to_string(), 1000);
        let mut world = MockWorld::from_scenario(bare);
        world.step(1.0); // 1 full day at speed 1

        let expected_water = 1000.0 - 13.0 * WATER_PER_BEAVER_DAY * cons_mult;
        let expected_carrot = 1000.0 - 13.0 * FOOD_PER_BEAVER_DAY * cons_mult;
        assert!(
            (world.resources["Water"] - expected_water).abs() < 0.05,
            "{diff:?}: expected water {expected_water}, got {}",
            world.resources["Water"]
        );
        assert!(
            (world.resources["Carrot"] - expected_carrot).abs() < 0.05,
            "{diff:?}: expected carrot {expected_carrot}, got {}",
            world.resources["Carrot"]
        );
    }
}

#[test]
fn golden_starvation_death_threshold_matches_wiki_3_75_days() {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.buildings.clear();
    cfg.resources.insert("Water".to_string(), 5000);
    cfg.resources.insert("Berries".to_string(), 0);
    cfg.resources.insert("Carrot".to_string(), 0);

    let mut world = MockWorld::from_scenario(cfg);
    let initial_pop = world.beavers.len();
    // 3.5 days starving -> still alive (< 3.75d verified threshold)
    world.step(3.5);
    assert_eq!(world.beavers.len(), initial_pop);
    assert!(world.metrics.starvation_events >= 1);

    // Crossing 3.75 days -> first death occurs
    world.step(STARVATION_DEATH_DAYS - 3.5 + 0.1);
    assert_eq!(world.beavers.len(), initial_pop - 1);
    assert_eq!(world.metrics.deaths, 1);
}

#[test]
fn golden_water_pump_and_inventor_hourly_rates() {
    let mut cfg = ScenarioConfig {
        name: "RateCheck".to_string(),
        faction: "Folktails".to_string(),
        difficulty: Difficulty::Normal,
        adults: 2,
        children: 0,
        bots: 0,
        science: 0,
        resources: BTreeMap::from([
            ("Water".to_string(), 0),
            ("Carrot".to_string(), 500),
            ("Log".to_string(), 100),
        ]),
        buildings: vec![
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
        ],
        ..Default::default()
    };
    cfg.reservoir_water_depth = 1.5;

    let mut world = MockWorld::from_scenario(cfg);
    world.work_end_hours = 16;
    world.step(1.0);

    // 1 pump * 3.0 Water/hr * 16h = 48.0 Water produced, minus 2 beavers * 2.12 = 4.24 consumed
    let expected_water = WATER_PUMP_PER_HOUR * 16.0 - 2.0 * WATER_PER_BEAVER_DAY;
    assert!(
        (world.resources["Water"] - expected_water).abs() < 0.1,
        "expected {expected_water}, got {}",
        world.resources["Water"]
    );
    // 1 inventor * 1.0 SP/hr * 16h = 16 SP
    assert_eq!(world.science, (INVENTOR_SP_PER_HOUR * 16.0) as u32);
}

#[test]
fn golden_evaporation_and_floodgate_badtide_direction() {
    // 1. Drought evaporation: dammed/gated reservoir evaporates at 0.0534 m/day vs 0.200 m/day uncontained
    let mut gated_cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    gated_cfg.is_hazardous = true;
    gated_cfg.hazard_kind = HazardKind::Drought;
    gated_cfg.reservoir_water_depth = 1.5;
    for b in &mut gated_cfg.buildings {
        if b.name.contains("Floodgate") {
            b.floodgate_height = Some(1.0);
        }
    }
    let mut gated = MockWorld::from_scenario(gated_cfg.clone());
    gated.step(1.0);
    assert!(
        (gated.reservoir_water_depth - (1.5 - EVAP_WIDE_POOL_PER_DAY)).abs() < 0.005,
        "gated drought evap mismatch: {}",
        gated.reservoir_water_depth
    );

    let mut open_cfg = gated_cfg;
    for b in &mut open_cfg.buildings {
        if b.name.contains("Floodgate") {
            b.floodgate_height = Some(0.0);
        }
    }
    let mut open = MockWorld::from_scenario(open_cfg);
    open.step(1.0);
    assert!(
        (open.reservoir_water_depth - (1.5 - EVAP_NARROW_CHANNEL_PER_DAY)).abs() < 0.005,
        "uncontained drought evap mismatch: {}",
        open.reservoir_water_depth
    );

    // 2. Badtide direction: raised floodgate (>= 0.8) diverts badwater; height 0 lets badwater flood in
    let mut bt_raised = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    bt_raised.is_hazardous = true;
    bt_raised.hazard_kind = HazardKind::Badtide;
    for b in &mut bt_raised.buildings {
        if b.name.contains("Floodgate") {
            b.floodgate_height = Some(1.0);
        }
    }
    let mut w_raised = MockWorld::from_scenario(bt_raised);
    w_raised.step(1.0);
    assert!(w_raised.river_contamination < 0.1);

    let mut bt_lowered = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    bt_lowered.is_hazardous = true;
    bt_lowered.hazard_kind = HazardKind::Badtide;
    for b in &mut bt_lowered.buildings {
        if b.name.contains("Floodgate") {
            b.floodgate_height = Some(0.0);
        }
    }
    let mut w_lowered = MockWorld::from_scenario(bt_lowered);
    w_lowered.step(1.0);
    assert!(w_lowered.river_contamination >= 0.9);
}

#[test]
fn golden_iron_teeth_breeding_pod_5_water_5_berries_5_days() {
    let mut cfg = ScenarioConfig::preset("IronTeeth", Difficulty::Normal);
    cfg.buildings
        .retain(|b| b.name.starts_with("BreedingPod") || b.name.starts_with("DistrictCenter"));
    cfg.resources.insert("Water".to_string(), 500);
    cfg.resources.insert("Berries".to_string(), 500);
    let mut world = MockWorld::from_scenario(cfg);
    let initial_pop = world.beavers.len();

    // 1 pod over 5.0 days produces 1 kit (even with 0 housing beds!)
    world.step(BREEDING_POD_CYCLE_DAYS + 0.05);
    assert_eq!(world.beavers.len(), initial_pop + 1);
}

#[tokio::test]
async fn contract_csharp_json_shapes_409_401_413_and_fifo_write_queue() {
    let srv = RunningMockServer::start(
        MockServerConfig {
            http_port: 0,
            ws_port: 0,
            lever_port: None,
            auth_token: "test-secret".to_string(),
            initial_ready: false,
            tick_interval: None,
            max_body_bytes: 512,
            write_job_delay: Some(Duration::from_millis(2)),
            ..Default::default()
        },
        ScenarioConfig::preset("Folktails", Difficulty::Normal),
    )
    .await
    .expect("start mock server");

    let base = format!("http://{}", srv.http_addr);
    let client = reqwest::Client::new();

    // 1. 401 + WWW-Authenticate on missing token (except /api/ping)
    let ping = client.get(format!("{base}/api/ping")).send().await.unwrap();
    assert_eq!(ping.status(), StatusCode::OK);

    let unauth = client
        .get(format!("{base}/api/summary"))
        .send()
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        unauth
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok()),
        Some("Bearer realm=\"timberbot\"")
    );

    // 2. 409 game_not_ready while ready=false
    let gated = client
        .get(format!("{base}/api/summary"))
        .bearer_auth("test-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(gated.status(), StatusCode::CONFLICT);
    let gated_json: Value = gated.json().await.unwrap();
    assert_eq!(gated_json["error"], "game_not_ready");

    // Flip ready=true
    let ready_resp = client
        .post(format!("{base}/api/ready"))
        .bearer_auth("test-secret")
        .json(&json!({"ready": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(ready_resp.status(), StatusCode::OK);

    // 3. 413 body_too_large when POST body exceeds max_body_bytes (512 in this test)
    let huge = "x".repeat(1024);
    let too_big = client
        .post(format!("{base}/api/agent/message"))
        .bearer_auth("test-secret")
        .json(&json!({"message": huge}))
        .send()
        .await
        .unwrap();
    assert_eq!(too_big.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let too_big_json: Value = too_big.json().await.unwrap();
    assert!(too_big_json["error"]
        .as_str()
        .unwrap_or("")
        .contains("body_too_large"));

    // 4. Exact C# JSON field names on /api/tiles, /api/summary, /api/beavers, /api/buildings
    let tiles_val: Value = client
        .get(format!("{base}/api/tiles?x1=9&y1=12&x2=12&y2=14"))
        .bearer_auth("test-secret")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(tiles_val["region"].is_object());
    let first_tile = &tiles_val["tiles"][0];
    for key in [
        "x",
        "y",
        "terrain",
        "water",
        "badwater",
        "entrance",
        "seedling",
        "dead",
        "contaminated",
        "moist",
        "occupants",
    ] {
        assert!(
            first_tile.get(key).is_some(),
            "missing C# TileInfo field '{key}' in {first_tile}"
        );
    }

    let summary_val: Value = client
        .get(format!("{base}/api/summary"))
        .bearer_auth("test-secret")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(summary_val.get("trees").is_some());
    assert!(summary_val.get("wellbeing").is_some());

    let beavers_val: Value = client
        .get(format!("{base}/api/beavers"))
        .bearer_auth("test-secret")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(beavers_val["items"][0]["liftingCapacity"], 14);
    assert!(beavers_val["items"][0]["wellbeing"].is_f64());

    // 5. FIFO write-job queue executes concurrent POST mutations sequentially
    let before_jobs = srv.state.write_jobs_executed();
    let mut handles = Vec::new();
    for spd in [1u8, 2, 3, 1, 2] {
        let c = client.clone();
        let url = format!("{base}/api/speed");
        handles.push(tokio::spawn(async move {
            c.post(url)
                .bearer_auth("test-secret")
                .json(&json!({"speed": spd}))
                .send()
                .await
                .unwrap()
                .status()
        }));
    }
    for h in handles {
        assert_eq!(h.await.unwrap(), StatusCode::OK);
    }
    assert_eq!(srv.state.write_jobs_executed(), before_jobs + 5);

    srv.shutdown();
}
