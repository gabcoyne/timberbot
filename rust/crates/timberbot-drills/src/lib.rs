//! Layer 4 (Ryan Perry 4-layer model): staged drill suite running the 4-layer loop
//! (`timberbot-state` -> `timberbot-tactics` -> `timberbot-eval`) against `timberbot-mock`
//! with real time-lag, builder queues, and prerequisite chains.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use timberbot_client::{ClientConfig, TimberbotClient};
use timberbot_eval::{DecisionTrace, TraceLogger};
use timberbot_mock::data::Difficulty;
use timberbot_mock::sim::{HazardKind, ScenarioBuilding, ScenarioConfig};
use timberbot_mock::{MockServerConfig, RunningMockServer};
use timberbot_protocol::http::{Orientation, Priority};
use timberbot_state::{ColonyState, Commitment};
use timberbot_tactics::{run_four_layer_tick, OutcomeStatus, TacticId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DrillResult {
    pub name: String,
    pub description: String,
    pub passed: bool,
    pub steps_run: u32,
    pub final_day: u32,
    pub deaths: u32,
    pub contamination_events: u32,
    pub playbook_failures: u32,
    pub tactics_used: BTreeMap<String, u32>,
    pub checks: Vec<DrillCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DrillCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DrillSuiteReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate_pct: f64,
    pub drills: Vec<DrillResult>,
}

struct DrillRunOutcome {
    final_state: ColonyState,
    deaths: u32,
    contamination_events: u32,
    cycles_completed: u32,
    playbook_failures: u32,
    tactics_used: BTreeMap<String, u32>,
    steps_run: u32,
    max_intake_contam: f64,
    min_water_stock: i64,
    min_food_stock: i64,
}

async fn run_drill_sim(
    drill_name: &str,
    scenario: ScenarioConfig,
    steps: u32,
    days_per_step: f64,
    mut logger: Option<&mut TraceLogger>,
    pre_hook: impl FnOnce(&RunningMockServer),
) -> Result<DrillRunOutcome> {
    let server = RunningMockServer::start(
        MockServerConfig {
            listen_address: "127.0.0.1".to_string(),
            http_port: 0,
            ws_port: 0,
            lever_port: None,
            auth_token: String::new(),
            initial_ready: true,
            tick_interval: None,
            days_per_tick_speed1: days_per_step,
            ..Default::default()
        },
        scenario,
    )
    .await?;
    pre_hook(&server);

    let client = TimberbotClient::new(ClientConfig {
        host: "127.0.0.1".to_string(),
        http_port: server.http_addr.port(),
        ws_port: server.ws_addr.port(),
        auth_token: None,
    })?;

    let mut commitment: Option<Commitment> = None;
    let mut tactics_used: BTreeMap<String, u32> = BTreeMap::new();
    let mut playbook_failures = 0u32;
    let mut max_intake_contam = 0.0_f64;
    let mut min_water_stock = i64::MAX;
    let mut min_food_stock = i64::MAX;
    let mut last_state: Option<ColonyState> = None;

    for step_idx in 0..steps {
        let t0 = Instant::now();
        let (state, decision, outcome, next_commit) =
            run_four_layer_tick(&client, commitment.take()).await?;
        let latency_ms = t0.elapsed().as_millis() as u64;

        *tactics_used
            .entry(decision.tactic.as_str().to_string())
            .or_insert(0) += 1;
        if outcome.status == OutcomeStatus::Failed {
            playbook_failures += 1;
        }
        max_intake_contam = max_intake_contam.max(state.river_contamination);
        min_water_stock = min_water_stock.min(state.water_stock);
        min_food_stock = min_food_stock.min(state.food_stock);

        if let Some(ref mut log) = logger {
            let trace = DecisionTrace::build(
                drill_name,
                step_idx,
                state.clone(),
                &decision,
                outcome,
                latency_ms,
                500,
            );
            log.append(&trace)?;
        }

        commitment = next_commit;
        last_state = Some(state);
        server.state.step_sim(days_per_step);
    }

    // Final post-step state read
    let final_snap = timberbot_state::ModSnapshot::fetch(&client).await?;
    let final_state = ColonyState::from_snapshot(&final_snap, commitment);
    max_intake_contam = max_intake_contam.max(final_state.river_contamination);
    let metrics = server.state.world.lock().unwrap().metrics.clone();
    server.shutdown();

    let _ = last_state;
    Ok(DrillRunOutcome {
        final_state,
        deaths: metrics.deaths,
        contamination_events: metrics.contamination_events,
        cycles_completed: metrics.cycles_completed,
        playbook_failures,
        tactics_used,
        steps_run: steps,
        max_intake_contam,
        min_water_stock,
        min_food_stock,
    })
}

fn check(name: &str, passed: bool, detail: String) -> DrillCheck {
    DrillCheck {
        name: name.to_string(),
        passed,
        detail,
    }
}

/// Drill 1: 2 days before badtide with an UNBUILT floodgate queued behind a low-priority Lodge.
/// Tests lead-time urgency, construction queue prioritization, workday boost, and badwater diversion.
pub async fn drill_badtide_unbuilt_floodgate_2d(
    logger: Option<&mut TraceLogger>,
) -> Result<DrillResult> {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.name = "DrillBadtide2dUnbuilt".to_string();
    cfg.auto_cycle_hazards = false;
    cfg.cycle = 2;
    cfg.cycle_day = 11;
    cfg.temperate_weather_duration = 12; // 2 days until badtide!
    cfg.hazardous_weather_duration = 4;
    cfg.is_hazardous = false;
    cfg.hazard_kind = HazardKind::Badtide;
    cfg.science = 200;
    cfg.resources.insert("Water".to_string(), 160);
    cfg.resources.insert("Carrot".to_string(), 180);
    cfg.resources.insert("Log".to_string(), 50);
    cfg.resources.insert("Plank".to_string(), 20);
    // Replace finished floodgate with an UNFINISHED floodgate + a decoy Lodge ahead of it
    cfg.buildings.retain(|b| !b.name.contains("Floodgate"));
    cfg.buildings.push(ScenarioBuilding {
        name: "Lodge.Folktails".to_string(),
        x: 18,
        y: 18,
        z: 2,
        orientation: Orientation::South,
        finished: false,
        paused: false,
        workers: None,
        floodgate_height: None,
        max_height: None,
        recipe: None,
        allowed_good: None,
        storage_mode: None,
    });
    cfg.buildings.push(ScenarioBuilding {
        name: "Floodgate.Folktails".to_string(),
        x: 10,
        y: 12,
        z: 1,
        orientation: Orientation::South,
        finished: false,
        paused: false,
        workers: None,
        floodgate_height: Some(0.0),
        max_height: Some(1.0),
        recipe: None,
        allowed_good: None,
        storage_mode: None,
    });

    // Run 30 steps * 0.25d = 7.5 days (2d warning + 4d badtide + 1.5d temperate recovery)
    let out = run_drill_sim(
        "badtide_unbuilt_floodgate_2d",
        cfg,
        30,
        0.25,
        logger,
        |srv| {
            // Ensure notifications advertise BadtideApproaching right at drill start
            let mut w = srv.state.world.lock().unwrap();
            w.notifications
                .push(timberbot_protocol::http::NotificationItem {
                    subject: "BadtideApproaching".to_string(),
                    description: "Badtide begins in 2 days".to_string(),
                    cycle: 2,
                    cycle_day: 11,
                    entity_id: None,
                });
            // Put Lodge at High priority so the agent MUST reprioritize Floodgate to VeryHigh!
            for b in &mut w.buildings {
                if b.name.starts_with("Lodge.") && b.finished == 0 {
                    b.construction_priority = Some(Priority::High);
                }
            }
        },
    )
    .await?;

    let picked_divert = out
        .tactics_used
        .get(TacticId::DivertBadtideFloodgates.as_str())
        .copied()
        .unwrap_or(0)
        > 0;
    let fg_finished = out.final_state.floodgates.iter().any(|f| f.finished);
    let checks = vec![
        check(
            "picked_divert_badtide_floodgates",
            picked_divert,
            format!("tactics={:?}", out.tactics_used),
        ),
        check(
            "floodgate_finished_in_time",
            fg_finished,
            format!("floodgates={:?}", out.final_state.floodgates),
        ),
        check(
            "intake_clean_during_badtide",
            out.max_intake_contam < 0.15,
            format!("max_intake_contam={:.2}", out.max_intake_contam),
        ),
        check(
            "zero_deaths_and_zero_contamination",
            out.deaths == 0 && out.contamination_events == 0,
            format!(
                "deaths={}, contam_events={}",
                out.deaths, out.contamination_events
            ),
        ),
        check(
            "zero_playbook_failures",
            out.playbook_failures == 0,
            format!("failures={}", out.playbook_failures),
        ),
    ];
    let passed = checks.iter().all(|c| c.passed);
    Ok(DrillResult {
        name: "badtide_unbuilt_floodgate_2d".to_string(),
        description: "2 days before badtide with unbuilt floodgate behind a High-priority Lodge"
            .to_string(),
        passed,
        steps_run: out.steps_run,
        final_day: out.final_state.day,
        deaths: out.deaths,
        contamination_events: out.contamination_events,
        playbook_failures: out.playbook_failures,
        tactics_used: out.tactics_used,
        checks,
    })
}

/// Drill 2: Low logs, few remaining trees, 0 planks, and no Forester.
/// Tests strict precedence: PowerWheel -> LumberMill -> Planks >= 7 -> Unlock & place Forester -> saplings planted!
pub async fn drill_low_logs_no_forester(logger: Option<&mut TraceLogger>) -> Result<DrillResult> {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.name = "DrillLowLogsNoForester".to_string();
    cfg.temperate_weather_duration = 25;
    cfg.cycle_day = 2;
    cfg.science = 50; // Enough to unlock Forester (30 SP) once planks exist
    cfg.initial_trees = 6; // Very few mature trees left!
    cfg.resources.insert("Water".to_string(), 220);
    cfg.resources.insert("Carrot".to_string(), 240);
    cfg.resources.insert("Log".to_string(), 42);
    cfg.resources.insert("Plank".to_string(), 0);
    // Keep Dam so agent focuses straight on timber/plank/forester chain
    cfg.buildings.push(ScenarioBuilding {
        name: "Dam.Folktails".to_string(),
        x: 10,
        y: 18,
        z: 1,
        orientation: Orientation::South,
        finished: true,
        paused: false,
        workers: None,
        floodgate_height: None,
        max_height: None,
        recipe: None,
        allowed_good: None,
        storage_mode: None,
    });
    cfg.buildings.retain(|b| {
        !b.name.contains("Forester")
            && !b.name.contains("LumberMill")
            && !b.name.contains("PowerWheel")
            && !b.name.contains("WaterWheel")
    });

    // Run 24 steps * 0.5d = 12 days so PowerWheel -> LumberMill -> Planks -> Forester -> Saplings all execute!
    let out = run_drill_sim("low_logs_no_forester", cfg, 24, 0.5, logger, |_| {}).await?;

    let staged_planks = out
        .tactics_used
        .get(TacticId::StagePlankChain.as_str())
        .copied()
        .unwrap_or(0)
        > 0;
    let unlocked_forester = out
        .tactics_used
        .get(TacticId::UnlockAndPlantForester.as_str())
        .copied()
        .unwrap_or(0)
        > 0;
    let checks = vec![
        check(
            "staged_plank_and_power_chain_first",
            staged_planks && out.final_state.power_generators_finished > 0,
            format!(
                "power_gens={}, lumber_mills={}, tactics={:?}",
                out.final_state.power_generators_finished,
                out.final_state.lumber_mills_finished,
                out.tactics_used
            ),
        ),
        check(
            "unlocked_and_built_forester",
            unlocked_forester && out.final_state.foresters_finished > 0,
            format!(
                "foresters_finished={}, unlocked={}",
                out.final_state.foresters_finished, out.final_state.forester_unlocked
            ),
        ),
        check(
            "forester_planted_new_saplings",
            out.final_state.sapling_trees > 0,
            format!(
                "saplings={}, grown_trees={}",
                out.final_state.sapling_trees, out.final_state.grown_trees
            ),
        ),
        check(
            "zero_deaths_and_zero_playbook_failures",
            out.deaths == 0 && out.playbook_failures == 0,
            format!("deaths={}, failures={}", out.deaths, out.playbook_failures),
        ),
    ];
    let passed = checks.iter().all(|c| c.passed);
    Ok(DrillResult {
        name: "low_logs_no_forester".to_string(),
        description: "Low logs, 6 remaining trees, 0 planks, no Forester — stages power -> planks -> Forester -> saplings".to_string(),
        passed,
        steps_run: out.steps_run,
        final_day: out.final_state.day,
        deaths: out.deaths,
        contamination_events: out.contamination_events,
        playbook_failures: out.playbook_failures,
        tactics_used: out.tactics_used,
        checks,
    })
}

/// Drill 3: Mid-drought food crunch with dry river, paused Farmhouse, and < 24h food runway.
pub async fn drill_mid_drought_food_crunch(
    logger: Option<&mut TraceLogger>,
) -> Result<DrillResult> {
    let mut cfg = ScenarioConfig::preset("Folktails", Difficulty::Normal);
    cfg.name = "DrillMidDroughtFoodCrunch".to_string();
    cfg.auto_cycle_hazards = false;
    cfg.is_hazardous = true;
    cfg.hazard_kind = HazardKind::Drought;
    cfg.cycle = 2;
    cfg.cycle_day = 15;
    cfg.temperate_weather_duration = 14;
    cfg.hazardous_weather_duration = 5;
    cfg.reservoir_water_depth = 0.0; // Dry river -> no new crop growth during drought!
    cfg.resources.insert("Water".to_string(), 220);
    cfg.resources.insert("Carrot".to_string(), 20);
    cfg.resources.insert("Berries".to_string(), 5);
    cfg.initial_crops = 30;
    cfg.initial_gatherables = 24;
    for b in &mut cfg.buildings {
        if b.name.contains("FarmHouse") || b.name.contains("Farmhouse") {
            b.paused = true;
        }
    }

    // 16 steps * 0.25d = 4.0 drought days
    let out = run_drill_sim("mid_drought_food_crunch", cfg, 16, 0.25, logger, |srv| {
        // Make existing crops already grown and ready to harvest once Farmhouse is unpaused!
        let mut w = srv.state.world.lock().unwrap();
        for c in &mut w.crops {
            c.growth = Some(1.0);
            c.grown = Some(1);
        }
    })
    .await?;

    let picked_food = out
        .tactics_used
        .get(TacticId::EmergencyFoodRationing.as_str())
        .copied()
        .unwrap_or(0)
        > 0;
    let checks = vec![
        check(
            "picked_emergency_food_rationing",
            picked_food,
            format!("tactics={:?}", out.tactics_used),
        ),
        check(
            "unpaused_farmhouse_and_harvested",
            out.final_state.farms_active > 0 && out.final_state.food_stock > 25,
            format!(
                "farms_active={}, food_stock={}",
                out.final_state.farms_active, out.final_state.food_stock
            ),
        ),
        check(
            "survived_drought_zero_deaths",
            out.deaths == 0 && out.playbook_failures == 0,
            format!(
                "deaths={}, min_food={}, failures={}",
                out.deaths, out.min_food_stock, out.playbook_failures
            ),
        ),
    ];
    let passed = checks.iter().all(|c| c.passed);
    Ok(DrillResult {
        name: "mid_drought_food_crunch".to_string(),
        description: "Mid-drought dry river with < 24h food runway and paused Farmhouse"
            .to_string(),
        passed,
        steps_run: out.steps_run,
        final_day: out.final_state.day,
        deaths: out.deaths,
        contamination_events: out.contamination_events,
        playbook_failures: out.playbook_failures,
        tactics_used: out.tactics_used,
        checks,
    })
}

/// Drill 4: Iron Teeth power-before-LumberMill precedence drill.
pub async fn drill_power_before_lumber_mill_it(
    logger: Option<&mut TraceLogger>,
) -> Result<DrillResult> {
    let mut cfg = ScenarioConfig::preset("IronTeeth", Difficulty::Normal);
    cfg.name = "DrillPowerBeforeMillIT".to_string();
    cfg.temperate_weather_duration = 20;
    cfg.resources.insert("Water".to_string(), 240);
    cfg.resources.insert("Kohlrabi".to_string(), 240);
    cfg.resources.insert("Berries".to_string(), 60);
    cfg.resources.insert("Log".to_string(), 55);
    cfg.resources.insert("Plank".to_string(), 0);
    cfg.buildings.push(ScenarioBuilding {
        name: "Dam.IronTeeth".to_string(),
        x: 10,
        y: 18,
        z: 1,
        orientation: Orientation::South,
        finished: true,
        paused: false,
        workers: None,
        floodgate_height: None,
        max_height: None,
        recipe: None,
        allowed_good: None,
        storage_mode: None,
    });
    cfg.buildings.push(ScenarioBuilding {
        name: "LumberMill.IronTeeth".to_string(),
        x: 19,
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
    });

    let out = run_drill_sim("power_before_lumber_mill_it", cfg, 16, 0.5, logger, |_| {}).await?;

    let checks = vec![
        check(
            "picked_stage_plank_chain",
            out.tactics_used
                .get(TacticId::StagePlankChain.as_str())
                .copied()
                .unwrap_or(0)
                > 0,
            format!("tactics={:?}", out.tactics_used),
        ),
        check(
            "built_large_power_wheel_and_powered_mill",
            out.final_state.power_generators_finished > 0
                && out.final_state.lumber_mills_unpowered == 0,
            format!(
                "gens={}, unpowered_mills={}, power={}/{}",
                out.final_state.power_generators_finished,
                out.final_state.lumber_mills_unpowered,
                out.final_state.power_supply,
                out.final_state.power_demand
            ),
        ),
        check(
            "produced_planks_from_logs",
            out.final_state.planks >= 10,
            format!("planks={}", out.final_state.planks),
        ),
        check(
            "zero_deaths_and_failures",
            out.deaths == 0 && out.playbook_failures == 0,
            format!("deaths={}, failures={}", out.deaths, out.playbook_failures),
        ),
    ];
    let passed = checks.iter().all(|c| c.passed);
    Ok(DrillResult {
        name: "power_before_lumber_mill_it".to_string(),
        description: "Iron Teeth unpowered LumberMill with 0 planks — stages LargePowerWheel first"
            .to_string(),
        passed,
        steps_run: out.steps_run,
        final_day: out.final_state.day,
        deaths: out.deaths,
        contamination_events: out.contamination_events,
        playbook_failures: out.playbook_failures,
        tactics_used: out.tactics_used,
        checks,
    })
}

/// Drill 5 & 6: Multi-cycle survival using the pure 4-layer loop (Folktails & Iron Teeth).
pub async fn drill_multi_cycle_survival(
    faction: &str,
    logger: Option<&mut TraceLogger>,
) -> Result<DrillResult> {
    let name = format!("multi_cycle_survival_{}", faction.to_ascii_lowercase());
    let mut cfg = ScenarioConfig::preset(faction, Difficulty::Normal);
    cfg.name = format!("DrillMultiCycle{faction}");
    // 60 steps * 0.5d = 30 in-game days (>= 1 full temperate + drought cycle!)
    let out = run_drill_sim(&name, cfg, 60, 0.5, logger, |_| {}).await?;

    let checks = vec![
        check(
            "completed_at_least_one_full_cycle",
            out.cycles_completed >= 1,
            format!(
                "cycles_completed={}, day={}",
                out.cycles_completed, out.final_state.day
            ),
        ),
        check(
            "zero_deaths",
            out.deaths == 0,
            format!(
                "deaths={}, pop={}, min_water={}, min_food={}",
                out.deaths, out.final_state.population, out.min_water_stock, out.min_food_stock
            ),
        ),
        check(
            "zero_playbook_failures",
            out.playbook_failures == 0,
            format!("failures={}", out.playbook_failures),
        ),
    ];
    let passed = checks.iter().all(|c| c.passed);
    Ok(DrillResult {
        name,
        description: format!("{faction} Normal 30-day multi-cycle survival under the 4-layer loop"),
        passed,
        steps_run: out.steps_run,
        final_day: out.final_state.day,
        deaths: out.deaths,
        contamination_events: out.contamination_events,
        playbook_failures: out.playbook_failures,
        tactics_used: out.tactics_used,
        checks,
    })
}

/// Run all 6 staged drills (or a substring-filtered subset), optionally writing every Layer 3
/// decision trace to `trace_out`.
pub async fn run_all_drills(
    filter: Option<&str>,
    trace_out: Option<&Path>,
) -> Result<DrillSuiteReport> {
    let mut logger = match trace_out {
        Some(p) => {
            let _ = std::fs::remove_file(p);
            Some(TraceLogger::open(p)?)
        }
        None => None,
    };

    let want = |name: &str| filter.is_none_or(|f| name.contains(f));
    let mut drills = Vec::new();

    if want("badtide_unbuilt_floodgate_2d") {
        drills.push(drill_badtide_unbuilt_floodgate_2d(logger.as_mut()).await?);
    }
    if want("low_logs_no_forester") {
        drills.push(drill_low_logs_no_forester(logger.as_mut()).await?);
    }
    if want("mid_drought_food_crunch") {
        drills.push(drill_mid_drought_food_crunch(logger.as_mut()).await?);
    }
    if want("power_before_lumber_mill_it") {
        drills.push(drill_power_before_lumber_mill_it(logger.as_mut()).await?);
    }
    if want("multi_cycle_survival_folktails") {
        drills.push(drill_multi_cycle_survival("Folktails", logger.as_mut()).await?);
    }
    if want("multi_cycle_survival_ironteeth") {
        drills.push(drill_multi_cycle_survival("IronTeeth", logger.as_mut()).await?);
    }

    let total = drills.len();
    let passed = drills.iter().filter(|d| d.passed).count();
    let failed = total.saturating_sub(passed);
    let pass_rate_pct = if total == 0 {
        100.0
    } else {
        ((passed as f64) / (total as f64) * 1000.0).round() / 10.0
    };

    Ok(DrillSuiteReport {
        total,
        passed,
        failed,
        pass_rate_pct,
        drills,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn all_staged_drills_pass_and_emit_valid_traces() {
        let trace_path = std::env::temp_dir().join(format!(
            "timberbot-drills-traces-{}.jsonl",
            std::process::id()
        ));
        let report = run_all_drills(None, Some(&trace_path))
            .await
            .expect("run staged drills");
        for d in &report.drills {
            assert!(d.passed, "drill '{}' failed: {:?}", d.name, d.checks);
        }
        assert_eq!(report.total, 6);
        assert_eq!(report.passed, 6);

        let traces = timberbot_eval::read_traces(&trace_path).expect("read emitted traces");
        assert!(
            traces.len() >= 150,
            "expected >= 150 trace rows across 6 drills"
        );
        let replay = timberbot_eval::replay_traces(&traces);
        assert_eq!(replay.decision_drift_count, 0);
        assert_eq!(replay.state_bug_count, 0);
        assert_eq!(replay.playbook_failure_count, 0);

        let _ = std::fs::remove_file(&trace_path);
    }
}
