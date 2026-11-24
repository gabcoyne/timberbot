use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use timberbot_client::ClientConfig;
use timberbot_mcp::{McpServer, McpServerConfig};
use timberbot_protocol::{DEFAULT_HTTP_PORT, DEFAULT_LEVER_PORT, DEFAULT_WS_PORT};

#[derive(Debug, Parser)]
#[command(
    name = "timberbot-mcp",
    about = "Standalone stdio MCP server wrapping Timberbot mod (:8085/:8086), official HTTP Levers (:8080), and read-only .timber save inspection"
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

    #[arg(
        long,
        default_value_t = format!("http://127.0.0.1:{DEFAULT_LEVER_PORT}"),
        env = "TIMBERBORN_LEVER_URL"
    )]
    lever_url: String,

    /// Optional root directory restricting `save_inspect` reads.
    #[arg(long, env = "TIMBERBORN_SAVE_ROOT")]
    save_root: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout remains clean JSON-RPC 2.0 framing.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let cfg = McpServerConfig {
        mod_client: ClientConfig {
            host: cli.host,
            http_port: cli.port,
            ws_port: cli.ws_port,
            auth_token: cli.auth_token.filter(|s| !s.trim().is_empty()),
        },
        lever_base_url: cli.lever_url,
        allowed_save_root: cli.save_root,
    };

    let server = McpServer::new(cfg)?;
    server.run_stdio().await
}
