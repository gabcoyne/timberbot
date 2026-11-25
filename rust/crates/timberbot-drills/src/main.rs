//! CLI for the 4-layer inner/outer loop: offline evals, staged drills, trace replay,
//! miss surfacing by layer, labeled case harvesting, and dual-gated tactic promotion.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use timberbot_drills::{run_all_drills_with_backend, run_promotion_gate};
use timberbot_eval::{
    evaluate_cases_with_backend, harvest_cases_from_traces, load_cases, read_traces, replay_traces,
    save_cases, surface_misses,
};
use timberbot_tactics::{ModelPolicyConfig, PolicyBackend};

#[derive(Debug, Clone, Args)]
struct BackendArgs {
    /// Optional OpenAI-compatible fast-model / Agent SDK endpoint URL for model-backed decide()
    #[arg(long, env = "TIMBERBOT_LLM_URL")]
    llm_url: Option<String>,
    /// Model name to send to the endpoint (e.g. grok-3-mini, claude-3-5-haiku, gpt-4.1-mini)
    #[arg(long, env = "TIMBERBOT_LLM_MODEL", default_value = "grok-3-mini")]
    llm_model: String,
    /// Optional bearer API key for the model endpoint
    #[arg(long, env = "TIMBERBOT_LLM_API_KEY")]
    llm_api_key: Option<String>,
    /// HTTP timeout in milliseconds before deterministic fallback
    #[arg(long, default_value_t = 1500)]
    llm_timeout_ms: u64,
    /// Staleness latency threshold in milliseconds before deterministic fallback
    #[arg(long, default_value_t = 1200)]
    llm_stale_ms: u64,
    /// Optional strategic goal injected into the model prompt
    #[arg(long)]
    llm_goal: Option<String>,
}

impl BackendArgs {
    fn to_backend(&self) -> PolicyBackend {
        match self.llm_url.as_deref().filter(|s| !s.trim().is_empty()) {
            Some(url) => {
                let mut cfg = ModelPolicyConfig::new(url.trim(), self.llm_model.trim());
                cfg.api_key = self.llm_api_key.clone();
                cfg.timeout_ms = self.llm_timeout_ms;
                cfg.stale_threshold_ms = self.llm_stale_ms;
                cfg.goal = self.llm_goal.clone();
                PolicyBackend::ModelBacked(cfg)
            }
            None => PolicyBackend::DeterministicBaseline,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "timberbot-drills",
    about = "Layer 3/4 offline evals, staged drills, trace outer-loop, and promotion gate for Timberbot"
)]
struct Cli {
    /// Emit machine-readable JSON output
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run Layer 3 offline evaluation on labeled cases with NO live game
    Offline {
        /// Path to labeled JSON eval cases
        #[arg(long, default_value = "../scenarios/eval-cases.json")]
        cases: PathBuf,
        /// Optional path to write the OfflineEvalReport JSON (for later regression checks)
        #[arg(long)]
        out_report: Option<PathBuf>,
        #[command(flatten)]
        backend: BackendArgs,
    },
    /// Run Layer 4 staged drills against timberbot-mock and optionally write JSONL traces
    Drills {
        /// Optional substring filter to run one drill (e.g. badtide, forester, gears)
        #[arg(long)]
        filter: Option<String>,
        /// Optional output path for Layer 3 JSONL traces
        #[arg(long)]
        trace_out: Option<PathBuf>,
        #[command(flatten)]
        backend: BackendArgs,
    },
    /// Replay a recorded JSONL trace file offline against the current decide() policy
    Replay {
        /// Path to JSONL trace file
        #[arg(long)]
        traces: PathBuf,
    },
    /// Outer loop: surface misses from JSONL traces (and optional eval cases) grouped by layer
    SurfaceMisses {
        /// Path to JSONL trace file
        #[arg(long)]
        traces: PathBuf,
        /// Optional labeled eval cases JSON to include alongside trace misses
        #[arg(long)]
        cases: Option<PathBuf>,
    },
    /// Outer loop: generate labeled eval cases from failing/drifted/stale/fallback traces
    HarvestCases {
        /// Path to JSONL trace file
        #[arg(long)]
        traces: PathBuf,
        /// Optional existing eval cases JSON for deduplication
        #[arg(long)]
        existing_cases: Option<PathBuf>,
        /// Output path for generated LabeledCase JSON array
        #[arg(long)]
        out: PathBuf,
    },
    /// Outer loop gate: require BOTH offline evals (with 0 regressions) AND staged drills to pass
    Promote {
        /// Path to labeled JSON eval cases
        #[arg(long, default_value = "../scenarios/eval-cases.json")]
        cases: PathBuf,
        /// Optional baseline OfflineEvalReport JSON to enforce zero regressions
        #[arg(long)]
        baseline_report: Option<PathBuf>,
        /// Optional substring filter for staged drills
        #[arg(long)]
        filter: Option<String>,
        /// Optional output path for Layer 3 JSONL traces
        #[arg(long)]
        trace_out: Option<PathBuf>,
        #[command(flatten)]
        backend: BackendArgs,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Offline {
            cases,
            out_report,
            backend,
        } => {
            let loaded = load_cases(&cases)?;
            let policy_backend = backend.to_backend();
            let report = evaluate_cases_with_backend(&loaded, &policy_backend).await;
            if let Some(ref p) = out_report {
                if let Some(parent) = p.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                std::fs::write(p, format!("{}\n", serde_json::to_string_pretty(&report)?))?;
            }
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Offline Eval: {}/{} passed ({:.1}%)",
                    report.passed, report.total, report.accuracy_pct
                );
                if !report.misses_by_kind.is_empty() {
                    println!("Misses by layer: {:?}", report.misses_by_kind);
                }
                for r in &report.results {
                    let status = if r.passed { "PASS" } else { "FAIL" };
                    println!(
                        "  [{status}] {} — expected={} actual={} {}",
                        r.id,
                        r.expected.as_str(),
                        r.actual.as_str(),
                        r.miss_kind
                            .map(|k| format!("[{k:?}: {}]", r.detail))
                            .unwrap_or_default()
                    );
                }
            }
            if report.failed > 0 {
                std::process::exit(1);
            }
        }
        Command::Drills {
            filter,
            trace_out,
            backend,
        } => {
            let policy_backend = backend.to_backend();
            let report = run_all_drills_with_backend(
                filter.as_deref(),
                trace_out.as_deref(),
                &policy_backend,
            )
            .await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Staged Drills: {}/{} passed ({:.1}%)",
                    report.passed, report.total, report.pass_rate_pct
                );
                for d in &report.drills {
                    let status = if d.passed { "PASS" } else { "FAIL" };
                    println!(
                        "  [{status}] {} (day {}, deaths={}, contam={}, failures={})",
                        d.name, d.final_day, d.deaths, d.contamination_events, d.playbook_failures
                    );
                    for c in &d.checks {
                        let cs = if c.passed { "ok" } else { "FAIL" };
                        println!("      [{cs}] {}: {}", c.name, c.detail);
                    }
                }
            }
            if report.failed > 0 {
                std::process::exit(1);
            }
        }
        Command::Replay { traces } => {
            let rows = read_traces(&traces)?;
            let report = replay_traces(&rows);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Trace Replay: {} rows | drift={} | state_bugs={} | playbook_failures={} | stale={} | max_latency={}ms",
                    report.total_traces,
                    report.decision_drift_count,
                    report.state_bug_count,
                    report.playbook_failure_count,
                    report.stale_count,
                    report.max_latency_ms
                );
                println!("Tactic histogram: {:?}", report.tactic_histogram);
            }
            if report.state_bug_count > 0 || report.playbook_failure_count > 0 {
                std::process::exit(1);
            }
        }
        Command::SurfaceMisses { traces, cases } => {
            let rows = read_traces(&traces)?;
            let labeled = match cases {
                Some(ref p) => Some(load_cases(p)?),
                None => None,
            };
            let report = surface_misses(&rows, labeled.as_deref());
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Surfaced Misses: {} total across {} traces (stale={}, fallbacks={})",
                    report.total_misses,
                    report.total_traces,
                    report.stale_traces,
                    report.fallback_traces
                );
                println!("  By layer: {:?}", report.misses_by_layer);
                for m in &report.misses {
                    println!(
                        "  [{:?}] {}#{} (day {}) picked={} suggested={} — {}",
                        m.layer,
                        m.source,
                        m.step_index,
                        m.day,
                        m.chosen_tactic.as_str(),
                        m.suggested_tactic.as_str(),
                        m.detail
                    );
                }
            }
        }
        Command::HarvestCases {
            traces,
            existing_cases,
            out,
        } => {
            let rows = read_traces(&traces)?;
            let existing = match existing_cases {
                Some(ref p) => load_cases(p)?,
                None => Vec::new(),
            };
            let harvested = harvest_cases_from_traces(&rows, &existing);
            save_cases(&out, &harvested)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "harvested_count": harvested.len(),
                        "output_path": out,
                        "ids": harvested.iter().map(|c| &c.id).collect::<Vec<_>>(),
                    }))?
                );
            } else {
                println!(
                    "Harvested {} labeled eval case(s) -> {}",
                    harvested.len(),
                    out.display()
                );
            }
        }
        Command::Promote {
            cases,
            baseline_report,
            filter,
            trace_out,
            backend,
        } => {
            let policy_backend = backend.to_backend();
            let report = run_promotion_gate(
                &cases,
                baseline_report.as_deref(),
                filter.as_deref(),
                trace_out.as_deref(),
                &policy_backend,
            )
            .await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                let verdict = if report.promoted {
                    "PROMOTED"
                } else {
                    "BLOCKED"
                };
                println!(
                    "Promotion Gate: {verdict} | Offline {}/{} ({:.1}%) | Drills {}/{} ({:.1}%)",
                    report.offline.passed,
                    report.offline.total,
                    report.offline.accuracy_pct,
                    report.drills.passed,
                    report.drills.total,
                    report.drills.pass_rate_pct
                );
                if let Some(ref reg) = report.regression {
                    println!(
                        "  Regression check: delta={:+.1}% newly_passing={:?} newly_failing={:?}",
                        reg.accuracy_delta_pct, reg.newly_passing, reg.newly_failing
                    );
                }
                for reason in &report.block_reasons {
                    println!("  BLOCK: {reason}");
                }
            }
            if !report.promoted {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
