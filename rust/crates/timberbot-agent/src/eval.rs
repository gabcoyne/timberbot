use anyhow::Result;
use serde::{Deserialize, Serialize};
use timberbot_client::{ClientConfig, TimberbotClient};
use timberbot_mock::data::Difficulty;
use timberbot_mock::sim::ScenarioConfig;
use timberbot_mock::{MockServerConfig, RunningMockServer};

use crate::{AgentLoop, AgentLoopConfig};

/// Structured evaluation metrics for one `(faction, difficulty)` survival run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalRunReport {
    pub faction: String,
    pub difficulty: String,
    pub target_cycles: u32,
    pub days_simulated: u32,
    pub cycles_completed: u32,
    pub droughts_survived: u32,
    pub badtides_survived: u32,
    pub initial_population: u32,
    pub final_population: u32,
    pub peak_population: u32,
    pub min_population: u32,
    pub deaths: u32,
    pub starvation_events: u32,
    pub thirst_events: u32,
    pub contamination_events: u32,
    pub final_water: i64,
    pub final_food: i64,
    pub decision_cycles: usize,
    pub passed: bool,
}

/// Full 6-scenario evaluation matrix across both factions and all three difficulties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuiteReport {
    pub runs: Vec<EvalRunReport>,
    pub all_passed: bool,
}

impl EvalSuiteReport {
    pub fn to_markdown_table(&self) -> String {
        let mut out = String::from(
            "| Faction | Difficulty | Days | Cycles | Droughts | Badtides | Pop (Init→Final) | Deaths | Starv | Thirst | Contam | Final Water | Final Food | Pass |\n\
             | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |\n",
        );
        for r in &self.runs {
            out.push_str(&format!(
                "| {} | {} | {} | {}/{} | {} | {} | {}→{} | {} | {} | {} | {} | {} | {} | {} |\n",
                r.faction,
                r.difficulty,
                r.days_simulated,
                r.cycles_completed,
                r.target_cycles,
                r.droughts_survived,
                r.badtides_survived,
                r.initial_population,
                r.final_population,
                r.deaths,
                r.starvation_events,
                r.thirst_events,
                r.contamination_events,
                r.final_water,
                r.final_food,
                if r.passed { "PASS" } else { "FAIL" }
            ));
        }
        out
    }
}

/// Run a single deterministic multi-cycle survival evaluation against the faithful mock.
pub async fn run_eval(
    faction: &str,
    difficulty: Difficulty,
    target_cycles: u32,
) -> Result<EvalRunReport> {
    let scenario = ScenarioConfig::preset(faction, difficulty);
    let initial_population = scenario.adults + scenario.children + scenario.bots;

    let server = RunningMockServer::start(
        MockServerConfig {
            listen_address: "127.0.0.1".to_string(),
            http_port: 0,
            ws_port: 0,
            lever_port: None,
            auth_token: String::new(),
            initial_ready: true,
            tick_interval: None,
            days_per_tick_speed1: 0.25,
            ..Default::default()
        },
        scenario,
    )
    .await?;

    let client = TimberbotClient::new(ClientConfig {
        host: "127.0.0.1".to_string(),
        http_port: server.http_addr.port(),
        ws_port: server.ws_addr.port(),
        auth_token: None,
    })?;

    let agent = AgentLoop::new(client.clone(), AgentLoopConfig::default());

    // Step 0.5 day per decision cycle (at speed 3 -> 0.5/3 base days per step)
    let max_steps = (target_cycles as usize) * 80;
    let mut decision_cycles = 0usize;

    for _ in 0..max_steps {
        let outcome = agent.run_cycle().await?;
        decision_cycles += 1;
        if outcome.population == 0 {
            break;
        }
        // Advance 0.5 simulated day at current speed (speed=3 -> base_days=1/6)
        server.state.step_sim(0.5 / 3.0);

        let completed = server
            .state
            .world
            .lock()
            .expect("lock")
            .metrics
            .cycles_completed;
        if completed >= target_cycles {
            break;
        }
    }

    let summary = client.summary().await?;
    let (metrics, day_number) = {
        let w = server.state.world.lock().expect("lock");
        (w.metrics.clone(), w.day_number)
    };
    let d0 = &summary.districts[0];
    let final_population = d0.population.total_beavers();
    let final_water = d0.resources.get("Water").copied().unwrap_or(0);
    let final_food = ["Carrot", "Kohlrabi", "FermentedCassava", "Berries"]
        .iter()
        .map(|k| d0.resources.get(*k).copied().unwrap_or(0))
        .sum::<i64>();

    let passed = metrics.cycles_completed >= target_cycles
        && metrics.droughts_survived >= 2
        && metrics.badtides_survived >= 1
        && metrics.deaths == 0
        && metrics.starvation_events == 0
        && metrics.thirst_events == 0
        && metrics.contamination_events == 0
        && final_population >= initial_population
        && final_water > 0
        && final_food > 0;

    server.shutdown();

    Ok(EvalRunReport {
        faction: faction.to_string(),
        difficulty: format!("{difficulty:?}"),
        target_cycles,
        days_simulated: day_number,
        cycles_completed: metrics.cycles_completed,
        droughts_survived: metrics.droughts_survived,
        badtides_survived: metrics.badtides_survived,
        initial_population,
        final_population,
        peak_population: metrics.peak_population,
        min_population: metrics.min_population,
        deaths: metrics.deaths,
        starvation_events: metrics.starvation_events,
        thirst_events: metrics.thirst_events,
        contamination_events: metrics.contamination_events,
        final_water,
        final_food,
        decision_cycles,
        passed,
    })
}

/// Run the full 6-cell `(Folktails | IronTeeth) x (Easy | Normal | Hard)` eval matrix.
/// Easy runs 6 cycles (badtides start on cycle 5), Normal and Hard run 5 cycles (badtides start on cycle 4/3),
/// so every cell must survive >= 2 droughts AND >= 1 badtide with zero deaths.
pub async fn run_full_eval_matrix() -> Result<EvalSuiteReport> {
    let matrix = [
        ("Folktails", Difficulty::Easy, 6),
        ("Folktails", Difficulty::Normal, 5),
        ("Folktails", Difficulty::Hard, 5),
        ("IronTeeth", Difficulty::Easy, 6),
        ("IronTeeth", Difficulty::Normal, 5),
        ("IronTeeth", Difficulty::Hard, 5),
    ];

    let mut runs = Vec::with_capacity(matrix.len());
    for (faction, difficulty, target_cycles) in matrix {
        runs.push(run_eval(faction, difficulty, target_cycles).await?);
    }
    let all_passed = runs.iter().all(|r| r.passed);
    Ok(EvalSuiteReport { runs, all_passed })
}
