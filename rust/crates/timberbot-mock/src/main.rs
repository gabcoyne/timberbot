use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use timberbot_mock::sim::ScenarioConfig;
use timberbot_mock::{MockServerConfig, RunningMockServer};
use timberbot_protocol::{DEFAULT_HTTP_PORT, DEFAULT_LEVER_PORT, DEFAULT_WS_PORT};

#[derive(Debug, Parser)]
#[command(
    name = "timberbot-mock",
    about = "Mock Timberbot HTTP/WS + Lever server"
)]
struct Cli {
    /// Listen address (default 127.0.0.1; non-loopback requires --auth-token).
    #[arg(long, default_value = "127.0.0.1", env = "TBOT_LISTEN_ADDRESS")]
    listen: String,

    #[arg(long, default_value_t = DEFAULT_HTTP_PORT, env = "TBOT_PORT")]
    http_port: u16,

    #[arg(long, default_value_t = DEFAULT_WS_PORT, env = "TBOT_WS_PORT")]
    ws_port: u16,

    /// Port for the mock official HTTP Lever/Adapter API (0 to disable).
    #[arg(long, default_value_t = DEFAULT_LEVER_PORT)]
    lever_port: u16,

    #[arg(long, default_value = "", env = "TBOT_AUTH_TOKEN")]
    auth_token: String,

    /// Start with the ready gate already open (default false to mirror the real mod).
    #[arg(long, default_value_t = false)]
    ready: bool,

    /// Optional JSON scenario file to load.
    #[arg(long)]
    scenario: Option<PathBuf>,

    /// Background simulation tick interval in milliseconds (0 disables auto-tick).
    #[arg(long, default_value_t = 250)]
    tick_ms: u64,

    /// Simulated days per tick at speed 1.
    #[arg(long, default_value_t = 0.05)]
    days_per_tick: f64,
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
    let scenario = match cli.scenario {
        Some(path) => {
            let txt = std::fs::read_to_string(&path)?;
            serde_json::from_str::<ScenarioConfig>(&txt)?
        }
        None => ScenarioConfig::default(),
    };

    let config = MockServerConfig {
        listen_address: cli.listen,
        http_port: cli.http_port,
        ws_port: cli.ws_port,
        lever_port: if cli.lever_port == 0 {
            None
        } else {
            Some(cli.lever_port)
        },
        auth_token: cli.auth_token,
        initial_ready: cli.ready,
        tick_interval: if cli.tick_ms == 0 {
            None
        } else {
            Some(Duration::from_millis(cli.tick_ms))
        },
        days_per_tick_speed1: cli.days_per_tick,
        ..Default::default()
    };

    let running = RunningMockServer::start(config, scenario).await?;
    tracing::info!(
        "timberbot-mock listening: http={} ws={} lever={:?}",
        running.http_addr,
        running.ws_addr,
        running.lever_addr
    );

    tokio::signal::ctrl_c().await?;
    running.shutdown();
    Ok(())
}
