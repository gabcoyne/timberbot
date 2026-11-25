use std::time::Duration;

use anyhow::Result;
use timberbot_client::TimberbotClient;
use timberbot_protocol::http::{
    CuttingAreaRequest, FindPlacementRequest, ListQuery, PlaceBuildingRequest, PlantCropRequest,
    SetPriorityRequest, TilesQuery,
};

use crate::managers::{default_orientation, plan_actions, Action, Observation};

#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Speed to restore when unpausing after a decision cycle (1..=3).
    pub run_speed: u8,
    /// Bounding box to query on `/api/tiles` for river water/contamination checks.
    pub river_box: TilesQuery,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            run_speed: 3,
            river_box: TilesQuery {
                x1: Some(8),
                y1: Some(10),
                x2: Some(14),
                y2: Some(18),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CycleOutcome {
    pub day_number: u32,
    pub is_hazardous: bool,
    pub population: u32,
    pub water: i64,
    pub food: i64,
    pub logs: i64,
    pub actions_planned: usize,
    pub actions_succeeded: usize,
    pub actions_failed: usize,
}

/// Reference pause -> observe -> plan -> act -> verify -> unpause agent loop.
pub struct AgentLoop {
    client: TimberbotClient,
    config: AgentLoopConfig,
}

impl AgentLoop {
    pub fn new(client: TimberbotClient, config: AgentLoopConfig) -> Self {
        Self { client, config }
    }

    /// Execute one complete pause -> observe -> plan -> act -> verify -> unpause cycle.
    pub async fn run_cycle(&self) -> Result<CycleOutcome> {
        // 1. PAUSE
        self.client.set_speed(0).await?;

        // 2. OBSERVE
        let obs = self.observe().await?;

        // 3. PLAN (deterministic managers: water, food, logs, housing)
        let actions = plan_actions(&obs);
        let actions_planned = actions.len();

        // 4. ACT (sequentially — never issue mutating game API calls in parallel)
        let mut actions_succeeded = 0usize;
        let mut actions_failed = 0usize;
        for action in &actions {
            match self.execute_action(action).await {
                Ok(()) => actions_succeeded += 1,
                Err(err) => {
                    tracing::warn!(?action, %err, "planned action failed");
                    actions_failed += 1;
                }
            }
        }

        // 5. VERIFY (re-observe state after mutations)
        let verified = self.observe().await?;

        // 6. UNPAUSE
        self.client
            .set_speed(self.config.run_speed.clamp(1, 3))
            .await?;

        Ok(CycleOutcome {
            day_number: verified.summary.time.day_number,
            is_hazardous: verified.summary.weather.is_hazardous,
            population: verified.total_beavers(),
            water: verified.resource("Water"),
            food: verified.total_food(),
            logs: verified.resource("Log"),
            actions_planned,
            actions_succeeded,
            actions_failed,
        })
    }

    /// Run `max_cycles` cycles with `sleep_between` real-time delay between cycles.
    pub async fn run_for_cycles(
        &self,
        max_cycles: usize,
        sleep_between: Duration,
    ) -> Result<Vec<CycleOutcome>> {
        let mut history = Vec::with_capacity(max_cycles);
        for _ in 0..max_cycles {
            let outcome = self.run_cycle().await?;
            history.push(outcome);
            if !sleep_between.is_zero() {
                tokio::time::sleep(sleep_between).await;
            }
        }
        Ok(history)
    }

    async fn observe(&self) -> Result<Observation> {
        let summary = self.client.summary().await?;
        let buildings = self
            .client
            .buildings(&ListQuery {
                limit: Some(0),
                ..Default::default()
            })
            .await?
            .items;
        let tiles = self.client.tiles(&self.config.river_box).await?;
        let wh = self.client.workhours().await?;
        Ok(Observation {
            summary,
            buildings,
            tiles,
            work_end_hours: wh.end_hours.round().clamp(1.0, 24.0) as u8,
        })
    }

    async fn execute_action(&self, action: &Action) -> Result<()> {
        match action {
            Action::SetFloodgate { id, height, .. } => {
                self.client.set_floodgate(*id, *height).await?;
            }
            Action::PauseBuilding { id, paused, .. } => {
                self.client.pause_building(*id, *paused).await?;
            }
            Action::SetPriority { id, priority, .. } => {
                self.client
                    .set_priority(&SetPriorityRequest {
                        id: *id,
                        priority: *priority,
                        priority_type: Some("workplace".to_string()),
                    })
                    .await?;
            }
            Action::SetWorkers { id, count, .. } => {
                self.client.set_workers(*id, *count).await?;
            }
            Action::SetWorkHours { end_hours, .. } => {
                self.client.set_workhours(*end_hours).await?;
            }
            Action::FindAndPlace { prefab, .. } => {
                let found = self
                    .client
                    .find_placement(&FindPlacementRequest {
                        prefab: prefab.clone(),
                        x1: 0,
                        y1: 0,
                        x2: 0,
                        y2: 0,
                        x: Some(16),
                        y: Some(16),
                        radius: Some(10),
                    })
                    .await?;
                if let Some(cand) = found.candidates.first() {
                    self.client
                        .place_building(&PlaceBuildingRequest {
                            prefab: prefab.clone(),
                            x: cand.x,
                            y: cand.y,
                            z: cand.z,
                            orientation: default_orientation(),
                        })
                        .await?;
                }
            }
            Action::PlantCrops {
                x1,
                y1,
                x2,
                y2,
                z,
                crop,
                ..
            } => {
                self.client
                    .plant_crop(&PlantCropRequest {
                        x1: *x1,
                        y1: *y1,
                        x2: *x2,
                        y2: *y2,
                        z: *z,
                        crop: crop.clone(),
                    })
                    .await?;
            }
            Action::MarkTrees {
                x1, y1, x2, y2, z, ..
            } => {
                self.client
                    .mark_trees(&CuttingAreaRequest {
                        x1: *x1,
                        y1: *y1,
                        x2: *x2,
                        y2: *y2,
                        z: *z,
                        marked: true,
                    })
                    .await?;
            }
        }
        Ok(())
    }
}
