//! Integration tests for the outer-loop promotion gate and model-backed drill execution.

use std::path::PathBuf;

use axum::{routing::post, Json, Router};
use timberbot_drills::{run_all_drills_with_backend, run_promotion_gate};
use timberbot_eval::{load_cases, read_traces, save_cases, surface_misses};
use timberbot_tactics::{DecisionSource, ModelPolicyConfig, PolicyBackend, TacticId};

fn repo_cases_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("repo root")
        .join("scenarios/eval-cases.json")
}

#[tokio::test]
async fn promotion_gate_passes_on_clean_suite_and_blocks_on_regression() {
    let cases_path = repo_cases_path();
    let gate = run_promotion_gate(
        &cases_path,
        None,
        Some("badtide_unbuilt_floodgate_2d"),
        None,
        &PolicyBackend::DeterministicBaseline,
    )
    .await
    .expect("run promotion gate");

    assert!(gate.promoted, "gate should pass: {:?}", gate.block_reasons);
    assert_eq!(gate.offline.passed, gate.offline.total);
    assert_eq!(gate.drills.passed, 1);

    // Save clean baseline report, then introduce a deliberately mislabeled case -> gate MUST BLOCK!
    let dir = std::env::temp_dir().join(format!("timberbot-gate-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let baseline_path = dir.join("baseline.json");
    std::fs::write(
        &baseline_path,
        serde_json::to_string_pretty(&gate.offline).unwrap(),
    )
    .unwrap();

    let mut mutated_cases = load_cases(&cases_path).unwrap();
    mutated_cases[0].expected_tactic = TacticId::HoldAndMonitor; // wrong label forces failure + regression
    let bad_cases_path = dir.join("bad-cases.json");
    save_cases(&bad_cases_path, &mutated_cases).unwrap();

    let blocked = run_promotion_gate(
        &bad_cases_path,
        Some(&baseline_path),
        Some("badtide_unbuilt_floodgate_2d"),
        None,
        &PolicyBackend::DeterministicBaseline,
    )
    .await
    .expect("run blocked promotion gate");

    assert!(!blocked.promoted);
    assert!(blocked.regression.as_ref().unwrap().regressed);
    assert!(blocked
        .block_reasons
        .iter()
        .any(|r| r.contains("offline eval gate failed")));
    assert!(blocked
        .block_reasons
        .iter()
        .any(|r| r.contains("regression gate failed")));

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn model_backed_drill_falls_back_safely_and_records_provenance_in_traces() {
    // Mock LLM returns an invalid hallucinated tactic so every tick exercises strict validation
    // + deterministic fallback and logs `DecisionSource::Fallback` in JSONL traces!
    let app = Router::new().route(
        "/v1/chat/completions",
        post(|| async {
            Json(serde_json::json!({
                "choices": [{
                    "message": {
                        "content": "{\"tactic\":\"invented_tactic\",\"rationale\":\"bad model output\"}"
                    }
                }]
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let cfg = ModelPolicyConfig::new(
        format!("http://{addr}/v1/chat/completions"),
        "hallucinating-local-mock",
    );
    let trace_path = std::env::temp_dir().join(format!(
        "timberbot-llm-drill-traces-{}.jsonl",
        std::process::id()
    ));

    let report = run_all_drills_with_backend(
        Some("badtide_unbuilt_floodgate_2d"),
        Some(&trace_path),
        &PolicyBackend::ModelBacked(cfg),
    )
    .await
    .expect("run model-backed drill");

    assert_eq!(
        report.passed, 1,
        "deterministic fallback must save the drill"
    );
    let traces = read_traces(&trace_path).unwrap();
    assert!(!traces.is_empty());
    assert!(traces
        .iter()
        .all(|t| matches!(t.decision_source, Some(DecisionSource::Fallback { .. }))));

    let surfaced = surface_misses(&traces, None);
    assert_eq!(surfaced.fallback_traces, traces.len());
    assert_eq!(surfaced.total_misses, 0);

    let _ = std::fs::remove_file(&trace_path);
}
