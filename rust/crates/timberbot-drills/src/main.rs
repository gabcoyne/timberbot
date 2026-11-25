//! CLI for the 4-layer offline eval, staged drill suite, and trace replay outer loop.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use timberbot_drills::run_all_drills;
use timberbot_eval::{evaluate_cases, load_cases, read_traces, replay_traces};

#[derive(Debug, Parser)]
#[command(
    name = "timberbot-drills",
    about = "Layer 3/4 offline evals, staged drills, and JSONL trace replay for Timberbot"
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
    /// Run Layer 3 offline evaluation on labeled cases using decide() with NO live game
    Offline {
        /// Path to labeled JSON eval cases
        #[arg(long, default_value = "../scenarios/eval-cases.json")]
        cases: PathBuf,
    },
    /// Run Layer 4 staged drills against timberbot-mock and optionally write JSONL traces
    Drills {
        /// Optional substring filter to run one drill (e.g. badtide, forester, drought)
        #[arg(long)]
        filter: Option<String>,
        /// Optional output path for Layer 3 JSONL traces
        #[arg(long)]
        trace_out: Option<PathBuf>,
    },
    /// Replay a recorded JSONL trace file offline against the current decide() policy
    Replay {
        /// Path to JSONL trace file
        #[arg(long)]
        traces: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Offline { cases } => {
            let loaded = load_cases(&cases)?;
            let report = evaluate_cases(&loaded);
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
        Command::Drills { filter, trace_out } => {
            let report = run_all_drills(filter.as_deref(), trace_out.as_deref()).await?;
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
    }

    Ok(())
}
