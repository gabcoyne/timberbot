use timberbot_agent::run_full_eval_matrix;

#[tokio::test]
async fn eval_matrix_survives_droughts_and_badtides_across_both_factions_and_all_difficulties() {
    let report = run_full_eval_matrix().await.expect("run full eval matrix");

    eprintln!("\n{}", report.to_markdown_table());

    assert_eq!(report.runs.len(), 6);
    for run in &report.runs {
        assert!(
            run.passed,
            "eval scenario {} / {} failed: {run:?}",
            run.faction, run.difficulty
        );
        assert_eq!(
            run.deaths, 0,
            "{} / {} had deaths",
            run.faction, run.difficulty
        );
        assert!(
            run.droughts_survived >= 2,
            "{} / {} only survived {} droughts",
            run.faction,
            run.difficulty,
            run.droughts_survived
        );
        assert!(
            run.badtides_survived >= 1,
            "{} / {} only survived {} badtides",
            run.faction,
            run.difficulty,
            run.badtides_survived
        );
    }
    assert!(report.all_passed);
}
