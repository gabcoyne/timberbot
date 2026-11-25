//! Verifies the checked-in `scenarios/eval-cases.json` labeled suite against `decide()`
//! with NO live game.

use std::path::PathBuf;
use timberbot_eval::{evaluate_cases, load_cases};

#[test]
fn checked_in_eval_cases_pass_100_percent_offline() {
    let cases_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("repo root")
        .join("scenarios/eval-cases.json");

    let cases = load_cases(&cases_path).expect("load scenarios/eval-cases.json");
    assert!(cases.len() >= 12, "expected >= 12 labeled offline cases");
    let report = evaluate_cases(&cases);
    for r in &report.results {
        assert!(
            r.passed,
            "case '{}' failed ({:?}): {}",
            r.id, r.miss_kind, r.detail
        );
    }
    assert_eq!(report.passed, report.total);
}
