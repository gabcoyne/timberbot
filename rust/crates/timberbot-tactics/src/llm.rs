//! Configurable fast-model / Agent SDK / MCP-friendly inner-loop `decide()` backend
//! with strict JSON schema validation, latency/staleness guardrails, and deterministic
//! fallback to the baseline policy [`crate::decide`].

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use timberbot_state::{ColonyState, Commitment};

use crate::{available_catalogue, decide, CatalogueEntry, Decision, TacticId};

/// Provenance of a single inner-loop decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecisionSource {
    Baseline,
    Model { model: String },
    Fallback { model: String, reason: String },
}

impl DecisionSource {
    pub fn is_fallback(&self) -> bool {
        matches!(self, Self::Fallback { .. })
    }
}

/// Configuration for the model-backed inner-loop `decide()` endpoint.
/// Supports any OpenAI-compatible `/v1/chat/completions` endpoint (fast model, Grok,
/// Agent SDK gateway, or local proxy) as well as a direct JSON response endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPolicyConfig {
    pub endpoint_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_stale_threshold_ms")]
    pub stale_threshold_ms: u64,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub goal: Option<String>,
}

const fn default_timeout_ms() -> u64 {
    1500
}

const fn default_stale_threshold_ms() -> u64 {
    1200
}

const fn default_max_tokens() -> u32 {
    256
}

impl ModelPolicyConfig {
    pub fn new(endpoint_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            model: model.into(),
            api_key: None,
            timeout_ms: default_timeout_ms(),
            stale_threshold_ms: default_stale_threshold_ms(),
            temperature: 0.0,
            max_tokens: default_max_tokens(),
            goal: None,
        }
    }
}

/// Inner-loop policy backend: either deterministic baseline or model-backed with fallback.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PolicyBackend {
    #[default]
    DeterministicBaseline,
    ModelBacked(ModelPolicyConfig),
}

/// Strict JSON contract that the model MUST return for each decision tick.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelDecisionPayload {
    pub tactic: String,
    pub rationale: String,
    #[serde(default)]
    pub commit_days: Option<u32>,
}

/// Result of running [`decide_with_backend`], including provenance, latency, and staleness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyDecision {
    pub decision: Decision,
    pub source: DecisionSource,
    pub latency_ms: u64,
    pub stale: bool,
}

/// Build the compact, MCP-friendly `(system_prompt, user_prompt)` pair sent to the inner-loop
/// fast model over `ColonyState::to_prompt_summary()` + `available_catalogue`.
pub fn build_decision_prompt(
    state: &ColonyState,
    catalogue: &[CatalogueEntry],
    goal: Option<&str>,
) -> (String, String) {
    let system = concat!(
        "You are Timberbot's fast inner-loop tactic selector for Timberborn 1.1.\n",
        "Choose EXACTLY ONE enabled tactic from the provided catalogue.\n",
        "Never pick a disabled tactic, never invent a tactic name, and respect lead-time & survival priorities.\n",
        "Respond with ONLY a strict JSON object matching this schema (no markdown, no extra keys):\n",
        "{\"tactic\":\"<tactic_id>\",\"rationale\":\"<concise reason>\",\"commit_days\":<optional integer 0..7>}"
    )
    .to_string();

    let enabled_cat: Vec<serde_json::Value> = catalogue
        .iter()
        .map(|e| {
            serde_json::json!({
                "tactic": e.tactic.as_str(),
                "enabled": e.enabled,
                "lead_time_days": e.lead_time_days,
                "feasible_before_hazard": e.feasible_before_hazard,
                "blocked_by": e.blocked_by,
                "description": e.description,
            })
        })
        .collect();

    let mut user = format!("STATE:\n{}\n", state.to_prompt_summary());
    if let Some(g) = goal.filter(|s| !s.trim().is_empty()) {
        user.push_str(&format!("\nGOAL:\n{}\n", g.trim()));
    }
    if let Some(commit) = &state.active_commitment {
        user.push_str(&format!(
            "\nACTIVE_COMMITMENT:\ntactic={} day={}/{} reason={}\n",
            commit.tactic, state.day, commit.target_day, commit.reason
        ));
    }
    user.push_str(&format!(
        "\nAVAILABLE_CATALOGUE:\n{}\n",
        serde_json::to_string_pretty(&enabled_cat).unwrap_or_else(|_| "[]".to_string())
    ));
    (system, user)
}

/// Strip optional markdown code fences around a JSON string.
fn extract_json_object(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest
            .strip_prefix("json")
            .or_else(|| rest.strip_prefix("JSON"))
            .unwrap_or(rest)
            .trim_start();
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim();
        }
    }
    trimmed
}

/// Strictly validate a raw JSON string returned by the model against `available_catalogue`.
pub fn validate_model_payload(
    raw: &str,
    state: &ColonyState,
    catalogue: &[CatalogueEntry],
) -> Result<Decision, String> {
    let json_str = extract_json_object(raw);
    let payload: ModelDecisionPayload =
        serde_json::from_str(json_str).map_err(|e| format!("schema validation failed: {e}"))?;

    if payload.rationale.trim().is_empty() {
        return Err("rationale must be non-empty".to_string());
    }
    if let Some(days) = payload.commit_days {
        if days > 7 {
            return Err(format!("commit_days {days} exceeds maximum 7"));
        }
    }
    let tactic = TacticId::from_name(payload.tactic.trim())
        .ok_or_else(|| format!("unknown/hallucinated tactic '{}'", payload.tactic))?;
    let entry = catalogue
        .iter()
        .find(|e| e.tactic == tactic)
        .ok_or_else(|| format!("tactic '{}' missing from catalogue", tactic.as_str()))?;
    if !entry.enabled {
        return Err(format!(
            "tactic '{}' is disabled in catalogue ({})",
            tactic.as_str(),
            entry.blocked_by.join("; ")
        ));
    }

    let commitment = payload
        .commit_days
        .filter(|&d| d > 0 && tactic != TacticId::HoldAndMonitor)
        .map(|days| Commitment {
            tactic: tactic.as_str().to_string(),
            started_day: state.day,
            target_day: state.day.saturating_add(days),
            reason: payload.rationale.trim().to_string(),
        });

    Ok(Decision {
        tactic,
        rationale: payload.rationale.trim().to_string(),
        lead_time_days: entry.lead_time_days,
        commitment,
        catalogue: catalogue.to_vec(),
    })
}

/// Execute inner-loop decision selection using either the deterministic baseline or a
/// configured fast-model HTTP endpoint with strict validation, staleness detection, and
/// automatic deterministic fallback.
pub async fn decide_with_backend(state: &ColonyState, backend: &PolicyBackend) -> PolicyDecision {
    match backend {
        PolicyBackend::DeterministicBaseline => {
            let t0 = Instant::now();
            let decision = decide(state);
            PolicyDecision {
                decision,
                source: DecisionSource::Baseline,
                latency_ms: t0.elapsed().as_millis() as u64,
                stale: false,
            }
        }
        PolicyBackend::ModelBacked(cfg) => decide_model_with_fallback(state, cfg).await,
    }
}

async fn decide_model_with_fallback(
    state: &ColonyState,
    cfg: &ModelPolicyConfig,
) -> PolicyDecision {
    let catalogue = available_catalogue(state);
    let (system, user) = build_decision_prompt(state, &catalogue, cfg.goal.as_deref());
    let t0 = Instant::now();

    let fallback = |reason: String, latency_ms: u64, stale: bool| PolicyDecision {
        decision: decide(state),
        source: DecisionSource::Fallback {
            model: cfg.model.clone(),
            reason,
        },
        latency_ms,
        stale,
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(cfg.timeout_ms.max(50)))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return fallback(
                format!("http client build error: {e}"),
                t0.elapsed().as_millis() as u64,
                false,
            );
        }
    };

    let body = serde_json::json!({
        "model": cfg.model,
        "temperature": cfg.temperature,
        "max_tokens": cfg.max_tokens,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });

    let mut req = client.post(&cfg.endpoint_url).json(&body);
    if let Some(key) = cfg.api_key.as_deref().filter(|k| !k.trim().is_empty()) {
        req = req.bearer_auth(key.trim());
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let latency_ms = t0.elapsed().as_millis() as u64;
            let is_timeout = e.is_timeout() || latency_ms > cfg.stale_threshold_ms;
            return fallback(format!("request error: {e}"), latency_ms, is_timeout);
        }
    };

    let status = resp.status();
    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            let latency_ms = t0.elapsed().as_millis() as u64;
            return fallback(
                format!("response read error: {e}"),
                latency_ms,
                latency_ms > cfg.stale_threshold_ms,
            );
        }
    };

    let latency_ms = t0.elapsed().as_millis() as u64;
    if !status.is_success() {
        return fallback(
            format!("model endpoint HTTP {}", status.as_u16()),
            latency_ms,
            latency_ms > cfg.stale_threshold_ms,
        );
    }

    if latency_ms > cfg.stale_threshold_ms {
        return fallback(
            format!(
                "stale model response ({latency_ms}ms > {}ms threshold)",
                cfg.stale_threshold_ms
            ),
            latency_ms,
            true,
        );
    }

    // Support OpenAI/Grok/Agent SDK chat completion envelope OR direct JSON object
    let content_str = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(val) => {
            if let Some(c) = val
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|s| s.as_str())
            {
                c.to_string()
            } else if val.get("tactic").is_some() {
                text
            } else {
                return fallback(
                    "missing choices[0].message.content or tactic field".to_string(),
                    latency_ms,
                    false,
                );
            }
        }
        Err(e) => {
            return fallback(format!("non-JSON HTTP body: {e}"), latency_ms, false);
        }
    };

    match validate_model_payload(&content_str, state, &catalogue) {
        Ok(decision) => PolicyDecision {
            decision,
            source: DecisionSource::Model {
                model: cfg.model.clone(),
            },
            latency_ms,
            stale: false,
        },
        Err(reason) => fallback(reason, latency_ms, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::post, Json, Router};
    use std::net::SocketAddr;
    use timberbot_state::{Faction, WeatherPhase};

    fn sample_state() -> ColonyState {
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
            irrigated_tiles: 35,
            contaminated_tiles: 0,
            population: 12,
            adults: 9,
            children: 3,
            water_stock: 220,
            water_capacity: 330,
            water_runway_hours: 207.5,
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
            grown_trees: 45,
            sapling_trees: 10,
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

    async fn spawn_mock_llm(app: Router) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        addr
    }

    #[tokio::test]
    async fn model_backed_decide_accepts_valid_enabled_tactic() {
        let app = Router::new().route(
            "/v1/chat/completions",
            post(|| async {
                Json(serde_json::json!({
                    "choices": [{
                        "message": {
                            "content": "{\"tactic\":\"secure_early_water\",\"rationale\":\"Top off tanks ahead of drought\",\"commit_days\":2}"
                        }
                    }]
                }))
            }),
        );
        let addr = spawn_mock_llm(app).await;
        let state = sample_state();
        let cfg = ModelPolicyConfig::new(
            format!("http://{addr}/v1/chat/completions"),
            "fast-test-model",
        );
        let out = decide_with_backend(&state, &PolicyBackend::ModelBacked(cfg)).await;
        assert_eq!(out.decision.tactic, TacticId::SecureEarlyWater);
        assert_eq!(
            out.source,
            DecisionSource::Model {
                model: "fast-test-model".to_string()
            }
        );
        assert!(!out.stale);
        assert_eq!(out.decision.commitment.as_ref().unwrap().target_day, 12);
    }

    #[tokio::test]
    async fn model_backed_decide_falls_back_on_hallucinated_or_disabled_tactic() {
        let app = Router::new().route(
            "/v1/chat/completions",
            post(|| async {
                Json(serde_json::json!({
                    "choices": [{
                        "message": {
                            "content": "{\"tactic\":\"build_space_elevator\",\"rationale\":\"Hallucinated tactic\"}"
                        }
                    }]
                }))
            }),
        );
        let addr = spawn_mock_llm(app).await;
        let state = sample_state();
        let cfg = ModelPolicyConfig::new(
            format!("http://{addr}/v1/chat/completions"),
            "hallucinating-model",
        );
        let out = decide_with_backend(&state, &PolicyBackend::ModelBacked(cfg)).await;
        assert_eq!(out.decision.tactic, decide(&state).tactic);
        assert!(matches!(
            out.source,
            DecisionSource::Fallback { ref reason, .. } if reason.contains("unknown/hallucinated")
        ));

        // Disabled tactic (Iron Teeth breeding pods for Folktails colony) also triggers fallback
        let app2 = Router::new().route(
            "/v1/chat/completions",
            post(|| async {
                Json(serde_json::json!({
                    "choices": [{
                        "message": {
                            "content": "{\"tactic\":\"manage_breeding_pods\",\"rationale\":\"Wrong faction\"}"
                        }
                    }]
                }))
            }),
        );
        let addr2 = spawn_mock_llm(app2).await;
        let cfg2 = ModelPolicyConfig::new(
            format!("http://{addr2}/v1/chat/completions"),
            "disabled-tactic-model",
        );
        let out2 = decide_with_backend(&state, &PolicyBackend::ModelBacked(cfg2)).await;
        assert_eq!(out2.decision.tactic, decide(&state).tactic);
        assert!(matches!(
            out2.source,
            DecisionSource::Fallback { ref reason, .. } if reason.contains("disabled")
        ));
    }

    #[tokio::test]
    async fn model_backed_decide_falls_back_on_stale_latency_and_http_error() {
        let slow_app = Router::new().route(
            "/v1/chat/completions",
            post(|| async {
                tokio::time::sleep(Duration::from_millis(90)).await;
                Json(serde_json::json!({
                    "choices": [{
                        "message": {
                            "content": "{\"tactic\":\"hold_and_monitor\",\"rationale\":\"Slow reply\"}"
                        }
                    }]
                }))
            }),
        );
        let addr = spawn_mock_llm(slow_app).await;
        let state = sample_state();
        let mut cfg =
            ModelPolicyConfig::new(format!("http://{addr}/v1/chat/completions"), "slow-model");
        cfg.stale_threshold_ms = 30;
        cfg.timeout_ms = 500;
        let out = decide_with_backend(&state, &PolicyBackend::ModelBacked(cfg)).await;
        assert!(out.stale);
        assert!(out.source.is_fallback());
        assert_eq!(out.decision.tactic, decide(&state).tactic);

        let err_app = Router::new().route(
            "/v1/chat/completions",
            post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "upstream error") }),
        );
        let err_addr = spawn_mock_llm(err_app).await;
        let cfg_err = ModelPolicyConfig::new(
            format!("http://{err_addr}/v1/chat/completions"),
            "failing-model",
        );
        let out_err = decide_with_backend(&state, &PolicyBackend::ModelBacked(cfg_err)).await;
        assert!(matches!(
            out_err.source,
            DecisionSource::Fallback { ref reason, .. } if reason.contains("500")
        ));
    }

    #[test]
    fn prompt_builder_includes_summary_catalogue_and_goal() {
        let state = sample_state();
        let cat = available_catalogue(&state);
        let (sys, user) = build_decision_prompt(&state, &cat, Some("Survive drought 3"));
        assert!(sys.contains("\"tactic\""));
        assert!(user.contains("FACTION=Folktails"));
        assert!(user.contains("Survive drought 3"));
        assert!(user.contains("hold_and_monitor"));
    }
}
