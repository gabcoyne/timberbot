//! Layer 3 (Ryan Perry 4-layer model): trace logging, offline evaluation of `decide()` with
//! no live game, and 4-layer miss classification:
//! - [`MissKind::StateBug`] — Layer 1 typed state broke an invariant or misreported the world.
//! - [`MissKind::MissingCatalogueTactic`] — Layer 2 expected tactic was absent/disabled in `available_catalogue`.
//! - [`MissKind::PickError`] — Layer 2 expected tactic WAS enabled, but `decide()` picked another tactic (Prompt/Goal/Policy error).
//! - [`MissKind::PlaybookBug`] — Layer 2 `decide()` chose the right tactic, but playbook execution failed.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use timberbot_state::ColonyState;
use timberbot_tactics::{
    decide, CatalogueEntry, Decision, OutcomeStatus, PlaybookOutcome, TacticId,
};

/// Single Layer 3 decision trace row written to JSONL during live or drill execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionTrace {
    pub timestamp_ms: u64,
    pub drill_or_session: String,
    pub step_index: u32,
    pub typed_state: ColonyState,
    pub available_catalogue: Vec<CatalogueEntry>,
    pub chosen_tactic: TacticId,
    pub rationale: String,
    pub playbook_outcome: PlaybookOutcome,
    /// `true` if `latency_ms` exceeded the stale threshold or state changed mid-decision.
    pub stale: bool,
    pub latency_ms: u64,
}

impl DecisionTrace {
    pub fn build(
        session: impl Into<String>,
        step_index: u32,
        typed_state: ColonyState,
        decision: &Decision,
        playbook_outcome: PlaybookOutcome,
        latency_ms: u64,
        stale_threshold_ms: u64,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            timestamp_ms,
            drill_or_session: session.into(),
            step_index,
            typed_state,
            available_catalogue: decision.catalogue.clone(),
            chosen_tactic: decision.tactic,
            rationale: decision.rationale.clone(),
            playbook_outcome,
            stale: latency_ms > stale_threshold_ms,
            latency_ms,
        }
    }
}

/// Append-only JSONL logger for Layer 3 traces.
pub struct TraceLogger {
    path: PathBuf,
    file: File,
}

impl TraceLogger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open trace log {}", path.display()))?;
        Ok(Self { path, file })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&mut self, trace: &DecisionTrace) -> Result<()> {
        let line = serde_json::to_string(trace)?;
        writeln!(self.file, "{line}")?;
        self.file.flush()?;
        Ok(())
    }
}

pub fn read_traces(path: impl AsRef<Path>) -> Result<Vec<DecisionTrace>> {
    let path = path.as_ref();
    let file = File::open(path).with_context(|| format!("read trace log {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row: DecisionTrace = serde_json::from_str(trimmed)
            .with_context(|| format!("parse trace JSONL {}:{}", path.display(), idx + 1))?;
        out.push(row);
    }
    Ok(out)
}

/// Ryan Perry's 4-layer miss taxonomy — isolates *which layer* broke before changing code.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MissKind {
    /// Layer 1: typed state failed structural invariants or omitted critical fields.
    StateBug,
    /// Layer 2 catalogue: expected tactic was missing or disabled in `available_catalogue`.
    MissingCatalogueTactic,
    /// Layer 2 prompt/goal/policy: expected tactic WAS enabled, but `decide()` chose another.
    PickError,
    /// Layer 2 playbook: right tactic was chosen, but deterministic execution failed.
    PlaybookBug,
}

/// Labeled offline eval case (serialized in `scenarios/eval-cases.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledCase {
    pub id: String,
    pub description: String,
    pub typed_state: ColonyState,
    pub expected_tactic: TacticId,
    /// Optional recorded outcome from a trace; when present and `Failed`, flags `PlaybookBug`.
    #[serde(default)]
    pub recorded_outcome: Option<PlaybookOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaseEvalResult {
    pub id: String,
    pub description: String,
    pub expected: TacticId,
    pub actual: TacticId,
    pub passed: bool,
    pub miss_kind: Option<MissKind>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfflineEvalReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub accuracy_pct: f64,
    pub misses_by_kind: BTreeMap<MissKind, usize>,
    pub results: Vec<CaseEvalResult>,
}

/// Classify a single `(state, decision, expected_tactic, optional_outcome)` into `None` (pass)
/// or `Some((MissKind, detail))`.
pub fn classify_decision(
    state: &ColonyState,
    decision: &Decision,
    expected: TacticId,
    outcome: Option<&PlaybookOutcome>,
) -> Option<(MissKind, String)> {
    // 1. Layer 1 check: state invariants
    let inv_errors = state.validate_invariants();
    if !inv_errors.is_empty() {
        return Some((
            MissKind::StateBug,
            format!("Layer 1 state invariant failed: {}", inv_errors.join("; ")),
        ));
    }

    // 2. Layer 2 catalogue + pick check
    if decision.tactic != expected {
        let expected_enabled = decision
            .catalogue
            .iter()
            .find(|e| e.tactic == expected)
            .is_some_and(|e| e.enabled);
        if !expected_enabled {
            let blockers = decision
                .catalogue
                .iter()
                .find(|e| e.tactic == expected)
                .map(|e| e.blocked_by.join(", "))
                .unwrap_or_else(|| "not in catalogue".to_string());
            return Some((
                MissKind::MissingCatalogueTactic,
                format!(
                    "expected '{}' was disabled/missing in catalogue ({}); picked '{}'",
                    expected.as_str(),
                    blockers,
                    decision.tactic.as_str()
                ),
            ));
        }
        return Some((
            MissKind::PickError,
            format!(
                "expected '{}' WAS enabled, but decide() picked '{}' ({})",
                expected.as_str(),
                decision.tactic.as_str(),
                decision.rationale
            ),
        ));
    }

    // 3. Layer 2 playbook execution check (when outcome is attached from a trace)
    if let Some(out) = outcome {
        if out.status == OutcomeStatus::Failed {
            return Some((
                MissKind::PlaybookBug,
                format!(
                    "tactic '{}' picked correctly, but playbook failed: {}",
                    expected.as_str(),
                    out.error.as_deref().unwrap_or("unknown error")
                ),
            ));
        }
    }

    None
}

/// Run offline evaluation on labeled cases using the EXACT same `timberbot_tactics::decide()`
/// function with NO live game or mock server.
pub fn evaluate_cases(cases: &[LabeledCase]) -> OfflineEvalReport {
    let mut results = Vec::with_capacity(cases.len());
    let mut misses_by_kind: BTreeMap<MissKind, usize> = BTreeMap::new();
    let mut passed = 0usize;

    for c in cases {
        let decision = decide(&c.typed_state);
        match classify_decision(
            &c.typed_state,
            &decision,
            c.expected_tactic,
            c.recorded_outcome.as_ref(),
        ) {
            None => {
                passed += 1;
                results.push(CaseEvalResult {
                    id: c.id.clone(),
                    description: c.description.clone(),
                    expected: c.expected_tactic,
                    actual: decision.tactic,
                    passed: true,
                    miss_kind: None,
                    detail: decision.rationale,
                });
            }
            Some((kind, detail)) => {
                *misses_by_kind.entry(kind).or_insert(0) += 1;
                results.push(CaseEvalResult {
                    id: c.id.clone(),
                    description: c.description.clone(),
                    expected: c.expected_tactic,
                    actual: decision.tactic,
                    passed: false,
                    miss_kind: Some(kind),
                    detail,
                });
            }
        }
    }

    let total = cases.len();
    let failed = total.saturating_sub(passed);
    let accuracy_pct = if total == 0 {
        100.0
    } else {
        ((passed as f64) / (total as f64) * 1000.0).round() / 10.0
    };

    OfflineEvalReport {
        total,
        passed,
        failed,
        accuracy_pct,
        misses_by_kind,
        results,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceReplayReport {
    pub total_traces: usize,
    pub decision_drift_count: usize,
    pub state_bug_count: usize,
    pub playbook_failure_count: usize,
    pub stale_count: usize,
    pub max_latency_ms: u64,
    pub tactic_histogram: BTreeMap<String, usize>,
}

/// Replay a JSONL trace log offline against the CURRENT `decide()` function to measure
/// decision drift, state bugs, playbook failures, and stale/latency ticks.
pub fn replay_traces(traces: &[DecisionTrace]) -> TraceReplayReport {
    let mut decision_drift_count = 0usize;
    let mut state_bug_count = 0usize;
    let mut playbook_failure_count = 0usize;
    let mut stale_count = 0usize;
    let mut max_latency_ms = 0u64;
    let mut tactic_histogram: BTreeMap<String, usize> = BTreeMap::new();

    for t in traces {
        if !t.typed_state.validate_invariants().is_empty() {
            state_bug_count += 1;
        }
        let replayed = decide(&t.typed_state);
        if replayed.tactic != t.chosen_tactic {
            decision_drift_count += 1;
        }
        if t.playbook_outcome.status == OutcomeStatus::Failed {
            playbook_failure_count += 1;
        }
        if t.stale {
            stale_count += 1;
        }
        max_latency_ms = max_latency_ms.max(t.latency_ms);
        *tactic_histogram
            .entry(t.chosen_tactic.as_str().to_string())
            .or_insert(0) += 1;
    }

    TraceReplayReport {
        total_traces: traces.len(),
        decision_drift_count,
        state_bug_count,
        playbook_failure_count,
        stale_count,
        max_latency_ms,
        tactic_histogram,
    }
}

pub fn load_cases(path: impl AsRef<Path>) -> Result<Vec<LabeledCase>> {
    let raw = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("read eval cases {}", path.as_ref().display()))?;
    let cases: Vec<LabeledCase> = serde_json::from_str(&raw)?;
    Ok(cases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use timberbot_state::{Faction, WeatherPhase};

    fn base_state() -> ColonyState {
        ColonyState {
            faction: Faction::Folktails,
            day: 10,
            cycle: 1,
            cycle_day: 10,
            work_hours: 16,
            active_builders: 2,
            weather_phase: WeatherPhase::Temperate,
            days_until_hazard: 6,
            hazard_duration_days: 5,
            river_depth: 1.5,
            river_contamination: 0.0,
            upstream_contamination: 0.0,
            irrigated_tiles: 30,
            contaminated_tiles: 0,
            population: 12,
            adults: 9,
            children: 3,
            water_stock: 200,
            water_capacity: 330,
            water_runway_hours: 188.7,
            food_stock: 200,
            food_runway_hours: 150.9,
            logs: 60,
            planks: 20,
            gears: 0,
            treated_planks: 0,
            metal_blocks: 0,
            berries: 40,
            science: 200,
            total_beds: 15,
            free_beds: 3,
            unemployed_adults: 2,
            worker_shortage: 0,
            idle_workers: 2,
            contaminated_beavers: 0,
            pumps_active: 1,
            pumps_paused: 0,
            farms_active: 1,
            farms_paused: 0,
            foresters_finished: 1,
            foresters_queued: 0,
            forester_unlocked: true,
            dams_finished: 1,
            dams_queued: 0,
            floodgate_unlocked: true,
            floodgates: Vec::new(),
            lumber_mills_finished: 1,
            lumber_mills_queued: 0,
            lumber_mills_unpowered: 0,
            power_generators_finished: 1,
            power_generators_queued: 0,
            inventors_finished: 1,
            power_supply: 50.0,
            power_demand: 50.0,
            grown_trees: 40,
            sapling_trees: 5,
            total_crops: 30,
            grown_crops: 10,
            breeding_pods_active: 0,
            breeding_pods_paused: 0,
            breeding_pods_queued: 0,
            gear_workshops_finished: 0,
            gear_workshops_queued: 0,
            gear_workshop_unlocked: false,
            unreachable_buildings: 0,
            disconnected_power_consumers: 0,
            power_network_count: 1,
            wellbeing_avg: 8.0,
            construction_queue: Vec::new(),
            blocked_prerequisites: Vec::new(),
            active_commitment: None,
        }
    }

    #[test]
    fn classifies_all_four_miss_kinds_accurately() {
        // 1. StateBug: negative runway hours
        let mut s_bug = base_state();
        s_bug.water_runway_hours = -5.0;
        let r1 = evaluate_cases(&[LabeledCase {
            id: "state-bug".to_string(),
            description: "broken negative runway".to_string(),
            typed_state: s_bug,
            expected_tactic: TacticId::HoldAndMonitor,
            recorded_outcome: None,
        }]);
        assert_eq!(r1.results[0].miss_kind, Some(MissKind::StateBug));

        // 2. MissingCatalogueTactic: expecting UnlockAndPlantForester when already built (disabled in catalogue)
        let r2 = evaluate_cases(&[LabeledCase {
            id: "missing-cat".to_string(),
            description: "forester already built so tactic disabled".to_string(),
            typed_state: base_state(),
            expected_tactic: TacticId::UnlockAndPlantForester,
            recorded_outcome: None,
        }]);
        assert_eq!(
            r2.results[0].miss_kind,
            Some(MissKind::MissingCatalogueTactic)
        );

        // 3. PickError: expecting SecureEarlyWater (enabled because logs >= 15) when healthy state picks HoldAndMonitor
        let r3 = evaluate_cases(&[LabeledCase {
            id: "pick-err".to_string(),
            description: "expected enabled tactic that policy did not pick".to_string(),
            typed_state: base_state(),
            expected_tactic: TacticId::SecureEarlyWater,
            recorded_outcome: None,
        }]);
        assert_eq!(r3.results[0].miss_kind, Some(MissKind::PickError));

        // 4. PlaybookBug: right tactic picked, but recorded outcome failed
        let r4 = evaluate_cases(&[LabeledCase {
            id: "playbook-bug".to_string(),
            description: "right pick but execution failed".to_string(),
            typed_state: base_state(),
            expected_tactic: TacticId::HoldAndMonitor,
            recorded_outcome: Some(PlaybookOutcome {
                tactic: TacticId::HoldAndMonitor,
                status: OutcomeStatus::Failed,
                steps_executed: Vec::new(),
                error: Some("simulated write failure".to_string()),
            }),
        }]);
        assert_eq!(r4.results[0].miss_kind, Some(MissKind::PlaybookBug));
    }

    #[test]
    fn jsonl_trace_logger_roundtrip_and_replay() {
        let dir = std::env::temp_dir().join(format!("timberbot-eval-test-{}", std::process::id()));
        let log_path = dir.join("traces.jsonl");
        let _ = std::fs::remove_file(&log_path);

        let s = base_state();
        let d = decide(&s);
        let trace = DecisionTrace::build(
            "unit-test",
            0,
            s,
            &d,
            PlaybookOutcome {
                tactic: d.tactic,
                status: OutcomeStatus::NoOp,
                steps_executed: Vec::new(),
                error: None,
            },
            12,
            250,
        );

        let mut logger = TraceLogger::open(&log_path).expect("open trace logger");
        logger.append(&trace).expect("append trace");
        drop(logger);

        let loaded = read_traces(&log_path).expect("read traces");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].chosen_tactic, TacticId::HoldAndMonitor);
        assert!(!loaded[0].stale);

        let replay = replay_traces(&loaded);
        assert_eq!(replay.total_traces, 1);
        assert_eq!(replay.decision_drift_count, 0);
        assert_eq!(replay.state_bug_count, 0);
        assert_eq!(replay.playbook_failure_count, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
