pub mod http_server;
pub mod lever_server;
pub mod sim;
pub mod ws_server;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Result};
use timberbot_protocol::{
    requires_auth_token, DEFAULT_HTTP_PORT, DEFAULT_LEVER_PORT, DEFAULT_WS_PORT,
};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::sim::{MockWorld, ScenarioConfig};
use timberbot_protocol::ws::ServerFrame;

#[derive(Debug, Clone)]
pub struct MockServerConfig {
    pub listen_address: String,
    pub http_port: u16,
    pub ws_port: u16,
    pub lever_port: Option<u16>,
    pub auth_token: String,
    pub initial_ready: bool,
    /// Real-time background simulation tick interval. `None` disables automatic background ticking
    /// (tests can step the simulation deterministically or enable fast ticking).
    pub tick_interval: Option<Duration>,
    /// Simulated days advanced per tick at speed 1 (scales linearly with speed 1/2/3).
    pub days_per_tick_speed1: f64,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            listen_address: "127.0.0.1".to_string(),
            http_port: DEFAULT_HTTP_PORT,
            ws_port: DEFAULT_WS_PORT,
            lever_port: Some(DEFAULT_LEVER_PORT),
            auth_token: String::new(),
            initial_ready: false,
            tick_interval: Some(Duration::from_millis(250)),
            days_per_tick_speed1: 0.05,
        }
    }
}

pub struct SharedMockState {
    pub world: Mutex<MockWorld>,
    pub auth_token: String,
    pub ws_tx: broadcast::Sender<ServerFrame>,
}

impl SharedMockState {
    pub fn new(scenario: ScenarioConfig, auth_token: String, initial_ready: bool) -> Arc<Self> {
        let (ws_tx, _) = broadcast::channel(512);
        let mut world = MockWorld::from_scenario(scenario);
        world.agent_state.ready = initial_ready;
        Arc::new(Self {
            world: Mutex::new(world),
            auth_token: auth_token.trim().to_string(),
            ws_tx,
        })
    }

    pub fn broadcast_events(&self, frames: Vec<ServerFrame>) {
        for frame in frames {
            let _ = self.ws_tx.send(frame);
        }
    }

    /// Advance the mock simulation by `days_at_speed_1` (scaled by current speed) and publish events.
    pub fn step_sim(&self, days_at_speed_1: f64) {
        let frames = {
            let mut world = self.world.lock().expect("mock world lock poisoned");
            world.step(days_at_speed_1)
        };
        self.broadcast_events(frames);
    }
}

pub struct RunningMockServer {
    pub http_addr: SocketAddr,
    pub ws_addr: SocketAddr,
    pub lever_addr: Option<SocketAddr>,
    pub state: Arc<SharedMockState>,
    tasks: Vec<JoinHandle<()>>,
}

impl RunningMockServer {
    pub async fn start(config: MockServerConfig, scenario: ScenarioConfig) -> Result<Self> {
        if requires_auth_token(&config.listen_address, &config.auth_token) {
            bail!(
                "refusing to start: listen_address='{}' is non-loopback but auth_token is empty",
                config.listen_address
            );
        }

        let state = SharedMockState::new(scenario, config.auth_token.clone(), config.initial_ready);

        let bind_ip = match config.listen_address.trim() {
            "" | "localhost" => "127.0.0.1",
            "+" | "*" => "0.0.0.0",
            other => other,
        };

        let http_listener =
            tokio::net::TcpListener::bind(format!("{bind_ip}:{}", config.http_port)).await?;
        let http_addr = http_listener.local_addr()?;

        let ws_listener =
            tokio::net::TcpListener::bind(format!("{bind_ip}:{}", config.ws_port)).await?;
        let ws_addr = ws_listener.local_addr()?;

        let mut tasks = Vec::new();

        let http_router = http_server::build_router(Arc::clone(&state));
        tasks.push(tokio::spawn(async move {
            let _ = axum::serve(http_listener, http_router).await;
        }));

        let ws_router = ws_server::build_router(Arc::clone(&state));
        tasks.push(tokio::spawn(async move {
            let _ = axum::serve(ws_listener, ws_router).await;
        }));

        let lever_addr = if let Some(lever_port) = config.lever_port {
            let lever_listener =
                tokio::net::TcpListener::bind(format!("{bind_ip}:{lever_port}")).await?;
            let addr = lever_listener.local_addr()?;
            let lever_router = lever_server::build_router(Arc::clone(&state));
            tasks.push(tokio::spawn(async move {
                let _ = axum::serve(lever_listener, lever_router).await;
            }));
            Some(addr)
        } else {
            None
        };

        if let Some(interval) = config.tick_interval {
            let sim_state = Arc::clone(&state);
            let days_per_tick = config.days_per_tick_speed1;
            tasks.push(tokio::spawn(async move {
                let mut ticker = tokio::time::interval(interval);
                loop {
                    ticker.tick().await;
                    sim_state.step_sim(days_per_tick);
                }
            }));
        }

        Ok(Self {
            http_addr,
            ws_addr,
            lever_addr,
            state,
            tasks,
        })
    }

    pub fn shutdown(self) {
        for task in self.tasks {
            task.abort();
        }
    }
}
