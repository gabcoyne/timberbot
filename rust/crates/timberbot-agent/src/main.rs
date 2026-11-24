use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use timberbot_agent::{AgentLoop, AgentLoopConfig};
use timberbot_client::{ClientConfig, TimberbotClient};
use timberbot_protocol::{DEFAULT_HTTP_PORT, DEFAULT_WS_PORT};

#[derive(Debug, Parser)]
#[command(
    name = "timberbot-agent",
    about = "Reference pause-observe-plan-act-verify-unpause agent loop for Timberbot"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1", env = "TBOT_HOST")]
    host: String,

    #[arg(long, default_value_t = DEFAULT_HTTP_PORT, env = "TBOT_PORT")]
    port: u16,

    #[arg(long, default_value_t = DEFAULT_WS_PORT, env = "TBOT_WS_PORT")]
    ws_port: u16,

    #[arg(long, env = "TBOT_AUTH_TOKEN")]
    auth_token: Option<String>,

    /// Number of decision cycles to execute.
    #[arg(long, default_value_t = 10)]
    cycles: usize,

    /// Milliseconds to let the game run between cycles.
    #[arg(long, default_value_t = 1000)]
    interval_ms: u64,

    /// Game speed when unpaused (1..=3).
    #[arg(long, default_value_t = 3)]
    speed: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let client = TimberbotClient::new(ClientConfig {
        host: cli.host,
        http_port: cli.port,
        ws_port: cli.ws_port,
        auth_token: cli.auth_token.filter(|s| !s.trim().is_empty()),
    })?;

    let runner = AgentLoop::new(
        client,
        AgentLoopConfig {
            run_speed: cli.speed,
            ..Default::default()
        },
    );

    let outcomes = runner
        .run_for_cycles(cli.cycles, Duration::from_millis(cli.interval_ms))
        .await?;

    for (i, o) in outcomes.iter().enumerate() {
        tracing::info!(
            cycle = i + 1,
            day = o.day_number,
            hazard = o.is_hazardous,
            pop = o.population,
            water = o.water,
            food = o.food,
            logs = o.logs,
            planned = o.actions_planned,
            ok = o.actions_succeeded,
            failed = o.actions_failed,
            "completed decision cycle"
        );
    }

    Ok(())
}
