use std::fs;
use std::path::PathBuf;

use timberbot_agent::{AgentLoop, AgentLoopConfig};
use timberbot_client::{ClientConfig, TimberbotClient};
use timberbot_mock::sim::ScenarioConfig;
use timberbot_mock::{MockServerConfig, RunningMockServer};
use timberbot_protocol::http::{ListQuery, Priority};

fn scenario_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("workspace crate is under repo/rust/crates/<crate>")
        .join("scenarios")
        .join(file)
}

fn load_scenario(file: &str) -> ScenarioConfig {
    let raw = fs::read_to_string(scenario_path(file)).expect("read scenario json");
    serde_json::from_str(&raw).expect("parse scenario json")
}

async fn start_scenario(file: &str) -> (RunningMockServer, TimberbotClient, AgentLoop) {
    let cfg = MockServerConfig {
        listen_address: "127.0.0.1".to_string(),
        http_port: 0,
        ws_port: 0,
        lever_port: Some(0),
        auth_token: String::new(),
        initial_ready: true,
        tick_interval: None,
        days_per_tick_speed1: 0.25,
    };
    let server = RunningMockServer::start(cfg, load_scenario(file))
        .await
        .expect("start mock server");
    let client = TimberbotClient::new(ClientConfig {
        host: "127.0.0.1".to_string(),
        http_port: server.http_addr.port(),
        ws_port: server.ws_addr.port(),
        auth_token: None,
    })
    .expect("create TimberbotClient");
    let agent = AgentLoop::new(client.clone(), AgentLoopConfig::default());
    (server, client, agent)
}

#[tokio::test]
async fn fresh_start_bootstraps_farmhouse_crops_and_housing() {
    let (server, client, agent) = start_scenario("fresh-start.json").await;

    for _ in 0..8 {
        let out = agent.run_cycle().await.expect("run agent cycle");
        assert_eq!(out.actions_failed, 0, "all planned actions should succeed");
        server.state.step_sim(0.25);
    }

    let buildings = client
        .buildings(&ListQuery {
            limit: Some(0),
            ..Default::default()
        })
        .await
        .unwrap()
        .items;
    assert!(
        buildings.iter().any(|b| b.name.contains("Farmhouse")),
        "fresh-start should place a Farmhouse"
    );
    assert!(
        buildings.iter().any(|b| b.name.contains("Lodge")),
        "fresh-start should build housing for homeless beavers"
    );

    let summary = client.summary().await.unwrap();
    let pop = summary.districts[0].population.total_beavers();
    assert_eq!(pop, 11, "all starting beavers should survive");
    assert!(
        summary
            .crops
            .as_ref()
            .map(|c| c.growing + c.ready)
            .unwrap_or(0)
            >= 12,
        "fresh-start should expand crop fields from initial 4"
    );

    server.shutdown();
}

#[tokio::test]
async fn incoming_drought_end_to_end_survival() {
    let (server, client, agent) = start_scenario("incoming-drought.json").await;

    let initial_pop = client.summary().await.unwrap().districts[0]
        .population
        .total_beavers();
    assert_eq!(initial_pop, 11);

    // Cycle 1: immediately prepare while approaching drought (day 10 of 12 temperate)
    let first = agent.run_cycle().await.expect("initial prep cycle");
    assert!(
        first.actions_planned > 0,
        "should plan drought prep actions"
    );
    assert_eq!(first.actions_failed, 0);

    // Verify floodgate was raised to retain reservoir depth and tank added
    let bldgs = client
        .buildings(&ListQuery {
            limit: Some(0),
            ..Default::default()
        })
        .await
        .unwrap()
        .items;
    let gate = bldgs
        .iter()
        .find(|b| b.floodgate == Some(1))
        .expect("floodgate present");
    assert!(
        gate.height.unwrap_or(0.0) >= 1.4,
        "floodgate should be raised before drought, got {:?}",
        gate.height
    );
    assert!(
        bldgs
            .iter()
            .filter(|b| b.name.contains("SmallTank"))
            .count()
            >= 2,
        "water_manager should expand tank storage before drought"
    );

    // Step across the full drought (days 10..18) running agent cycles every ~0.75 simulated days
    let mut saw_drought = false;
    let mut exited_drought_to_cycle_2 = false;
    for _ in 0..24 {
        server.state.step_sim(0.2);
        let outcome = agent.run_cycle().await.expect("agent survival cycle");
        if outcome.is_hazardous {
            saw_drought = true;
        }
        let weather = client.weather().await.unwrap();
        if saw_drought && !weather.is_hazardous && weather.cycle >= 2 {
            exited_drought_to_cycle_2 = true;
            break;
        }
    }

    assert!(saw_drought, "simulation must enter the drought phase");
    assert!(
        exited_drought_to_cycle_2,
        "simulation must survive the drought and enter cycle 2 temperate"
    );

    let final_summary = client.summary().await.unwrap();
    let d0 = &final_summary.districts[0];
    assert_eq!(
        d0.population.total_beavers(),
        initial_pop,
        "zero beaver deaths during drought survival"
    );
    let water = *d0.resources.get("Water").unwrap_or(&0);
    let food = d0.resources.get("Carrot").copied().unwrap_or(0)
        + d0.resources.get("Berries").copied().unwrap_or(0);
    assert!(
        water > 0,
        "colony must exit drought with positive water (got {water})"
    );
    assert!(
        food > 0,
        "colony must exit drought with positive food (got {food})"
    );

    // Verify floodgate lowers back to temperate height once drought ends
    let post_bldgs = client
        .buildings(&ListQuery {
            limit: Some(0),
            ..Default::default()
        })
        .await
        .unwrap()
        .items;
    let post_gate = post_bldgs.iter().find(|b| b.floodgate == Some(1)).unwrap();
    assert!(
        post_gate.height.unwrap_or(2.0) <= 0.6,
        "floodgate should return to temperate height after drought, got {:?}",
        post_gate.height
    );

    server.shutdown();
}

#[tokio::test]
async fn food_crisis_recovers_food_and_unpauses_farmhouse() {
    let (server, client, agent) = start_scenario("food-crisis.json").await;

    let before_food = {
        let s = client.summary().await.unwrap();
        let r = &s.districts[0].resources;
        r.get("Carrot").copied().unwrap_or(0) + r.get("Berries").copied().unwrap_or(0)
    };
    assert!(before_food < 30, "fixture should start in a food crisis");

    let first = agent.run_cycle().await.expect("crisis response cycle");
    assert_eq!(first.actions_failed, 0);

    let bldgs = client
        .buildings(&ListQuery {
            limit: Some(0),
            ..Default::default()
        })
        .await
        .unwrap()
        .items;
    let farm = bldgs
        .iter()
        .find(|b| b.name.contains("Farmhouse"))
        .expect("farmhouse present");
    assert_eq!(farm.paused, 0, "Farmhouse must be unpaused");
    assert_eq!(
        farm.workplace_priority,
        Some(Priority::VeryHigh),
        "Farmhouse priority must be VeryHigh during food crisis"
    );
    assert!(
        bldgs.iter().any(|b| b.name.contains("GathererFlag")),
        "food_manager should place a GathererFlag for immediate berries"
    );

    for _ in 0..8 {
        server.state.step_sim(0.25);
        agent.run_cycle().await.expect("recovery cycle");
    }

    let after = client.summary().await.unwrap();
    let d0 = &after.districts[0];
    let after_food = d0.resources.get("Carrot").copied().unwrap_or(0)
        + d0.resources.get("Berries").copied().unwrap_or(0);
    assert_eq!(d0.population.total_beavers(), 10, "zero starvation deaths");
    assert!(
        after_food > before_food * 2,
        "food should recover strongly (before={before_food}, after={after_food})"
    );

    server.shutdown();
}

#[tokio::test]
async fn badtide_pauses_pumps_raises_floodgate_and_resumes_after_clean_water() {
    let (server, client, agent) = start_scenario("badtide.json").await;

    // 1. Immediate reaction to active contaminated badtide
    let first = agent.run_cycle().await.expect("badtide reaction cycle");
    assert_eq!(first.actions_failed, 0);

    let bldgs = client
        .buildings(&ListQuery {
            limit: Some(0),
            ..Default::default()
        })
        .await
        .unwrap()
        .items;
    let pump = bldgs
        .iter()
        .find(|b| b.name.contains("WaterPump"))
        .expect("WaterPump present");
    assert_eq!(
        pump.paused, 1,
        "WaterPump must be paused while river is contaminated"
    );
    let gate = bldgs
        .iter()
        .find(|b| b.floodgate == Some(1))
        .expect("Floodgate present");
    assert!(
        gate.height.unwrap_or(0.0) >= 1.4,
        "Floodgate must be raised to block badwater inflow"
    );

    // 2. Step through the 3-day badtide and into clean temperate water
    let mut back_to_temperate = false;
    for _ in 0..20 {
        server.state.step_sim(0.2);
        agent.run_cycle().await.expect("badtide endure cycle");
        let w = client.weather().await.unwrap();
        if !w.is_hazardous && w.cycle >= 3 {
            back_to_temperate = true;
            break;
        }
    }
    assert!(back_to_temperate, "colony must exit badtide into cycle 3");

    // 3. After clean temperate water returns, WaterPump is unpaused and all beavers survived
    let post_bldgs = client
        .buildings(&ListQuery {
            limit: Some(0),
            ..Default::default()
        })
        .await
        .unwrap()
        .items;
    let post_pump = post_bldgs
        .iter()
        .find(|b| b.name.contains("WaterPump"))
        .unwrap();
    assert_eq!(
        post_pump.paused, 0,
        "WaterPump must resume once river contamination clears"
    );
    let summary = client.summary().await.unwrap();
    assert_eq!(
        summary.districts[0].population.total_beavers(),
        10,
        "all beavers should survive the badtide"
    );

    server.shutdown();
}
