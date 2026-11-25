//! End-of-run analytics and interactive replay generator (JSON + Markdown + self-contained HTML)
//! over Layer 3 JSONL [`DecisionTrace`] logs.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use timberbot_state::WeatherPhase;
use timberbot_tactics::{DecisionSource, OutcomeStatus};

use crate::{surface_misses, DecisionTrace, MissKind, SurfacedMiss};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TacticStat {
    pub tactic: String,
    pub count: usize,
    pub share_pct: f64,
    pub succeeded: usize,
    pub noop: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencySummary {
    pub min_ms: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub max_ms: u64,
    pub mean_ms: f64,
    pub stale_count: usize,
    pub stale_rate_pct: f64,
    pub source_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelinePoint {
    pub index: usize,
    pub session: String,
    pub step_index: u32,
    pub day: u32,
    pub cycle: u32,
    pub cycle_day: u32,
    pub weather_phase: WeatherPhase,
    pub population: u32,
    pub water_stock: i64,
    pub water_runway_hours: f64,
    pub food_stock: i64,
    pub food_runway_hours: f64,
    pub wellbeing_avg: f64,
    pub river_depth: f64,
    pub river_contamination: f64,
    pub tactic: String,
    pub outcome_status: OutcomeStatus,
    pub latency_ms: u64,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayFrame {
    pub index: usize,
    pub session: String,
    pub step_index: u32,
    pub day: u32,
    pub cycle: u32,
    pub weather_phase: WeatherPhase,
    pub population: u32,
    pub water_stock: i64,
    pub food_stock: i64,
    pub wellbeing_avg: f64,
    pub logs: i64,
    pub planks: i64,
    pub gears: i64,
    pub science: u32,
    pub river_depth: f64,
    pub river_contamination: f64,
    pub chosen_tactic: String,
    pub rationale: String,
    pub decision_source: String,
    pub latency_ms: u64,
    pub stale: bool,
    pub outcome_status: OutcomeStatus,
    pub steps_executed: Vec<String>,
    pub error: Option<String>,
    pub enabled_tactics: Vec<String>,
    pub prompt_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunAnalytics {
    pub total_steps: usize,
    pub sessions: Vec<String>,
    pub max_day: u32,
    pub min_population: u32,
    pub max_population: u32,
    pub min_water_stock: i64,
    pub min_food_stock: i64,
    pub mean_wellbeing: f64,
    pub tactic_distribution: Vec<TacticStat>,
    pub latency: LatencySummary,
    pub total_misses: usize,
    pub failure_attribution: BTreeMap<MissKind, usize>,
    pub surfaced_misses: Vec<SurfacedMiss>,
    pub timeline: Vec<TimelinePoint>,
    pub frames: Vec<ReplayFrame>,
}

/// Compute comprehensive end-of-run analytics and replay frames from recorded traces.
pub fn build_run_analytics(traces: &[DecisionTrace]) -> RunAnalytics {
    let total_steps = traces.len();
    let mut sessions_set = std::collections::BTreeSet::new();
    let mut max_day = 0u32;
    let mut min_pop = u32::MAX;
    let mut max_pop = 0u32;
    let mut min_water = i64::MAX;
    let mut min_food = i64::MAX;
    let mut wellbeing_sum = 0.0_f64;

    let mut tactic_acc: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();
    let mut latencies: Vec<u64> = Vec::with_capacity(total_steps);
    let mut stale_count = 0usize;
    let mut source_counts: BTreeMap<String, usize> = BTreeMap::new();

    let mut timeline = Vec::with_capacity(total_steps);
    let mut frames = Vec::with_capacity(total_steps);

    for (index, t) in traces.iter().enumerate() {
        sessions_set.insert(t.drill_or_session.clone());
        let s = &t.typed_state;
        max_day = max_day.max(s.day);
        min_pop = min_pop.min(s.population);
        max_pop = max_pop.max(s.population);
        min_water = min_water.min(s.water_stock);
        min_food = min_food.min(s.food_stock);
        wellbeing_sum += s.wellbeing_avg;

        latencies.push(t.latency_ms);
        if t.stale {
            stale_count += 1;
        }
        let source_label = match &t.decision_source {
            None | Some(DecisionSource::Baseline) => "baseline".to_string(),
            Some(DecisionSource::Model { model }) => format!("model:{model}"),
            Some(DecisionSource::Fallback { model, reason }) => {
                format!("fallback:{model} ({reason})")
            }
        };
        let source_bucket = match &t.decision_source {
            None | Some(DecisionSource::Baseline) => "baseline",
            Some(DecisionSource::Model { .. }) => "model",
            Some(DecisionSource::Fallback { .. }) => "fallback",
        };
        *source_counts.entry(source_bucket.to_string()).or_insert(0) += 1;

        let t_name = t.chosen_tactic.as_str().to_string();
        let entry = tactic_acc.entry(t_name.clone()).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        match t.playbook_outcome.status {
            OutcomeStatus::Succeeded | OutcomeStatus::Partial => entry.1 += 1,
            OutcomeStatus::NoOp | OutcomeStatus::Blocked => entry.2 += 1,
            OutcomeStatus::Failed => entry.3 += 1,
        }

        timeline.push(TimelinePoint {
            index,
            session: t.drill_or_session.clone(),
            step_index: t.step_index,
            day: s.day,
            cycle: s.cycle,
            cycle_day: s.cycle_day,
            weather_phase: s.weather_phase,
            population: s.population,
            water_stock: s.water_stock,
            water_runway_hours: s.water_runway_hours,
            food_stock: s.food_stock,
            food_runway_hours: s.food_runway_hours,
            wellbeing_avg: s.wellbeing_avg,
            river_depth: s.river_depth,
            river_contamination: s.river_contamination,
            tactic: t_name.clone(),
            outcome_status: t.playbook_outcome.status,
            latency_ms: t.latency_ms,
            stale: t.stale,
        });

        let enabled_tactics = t
            .available_catalogue
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.tactic.as_str().to_string())
            .collect();

        frames.push(ReplayFrame {
            index,
            session: t.drill_or_session.clone(),
            step_index: t.step_index,
            day: s.day,
            cycle: s.cycle,
            weather_phase: s.weather_phase,
            population: s.population,
            water_stock: s.water_stock,
            food_stock: s.food_stock,
            wellbeing_avg: s.wellbeing_avg,
            logs: s.logs,
            planks: s.planks,
            gears: s.gears,
            science: s.science,
            river_depth: s.river_depth,
            river_contamination: s.river_contamination,
            chosen_tactic: t_name,
            rationale: t.rationale.clone(),
            decision_source: source_label,
            latency_ms: t.latency_ms,
            stale: t.stale,
            outcome_status: t.playbook_outcome.status,
            steps_executed: t.playbook_outcome.steps_executed.clone(),
            error: t.playbook_outcome.error.clone(),
            enabled_tactics,
            prompt_summary: s.to_prompt_summary(),
        });
    }

    let mut tactic_distribution: Vec<TacticStat> = tactic_acc
        .into_iter()
        .map(|(tactic, (count, succeeded, noop, failed))| TacticStat {
            tactic,
            count,
            share_pct: pct(count, total_steps),
            succeeded,
            noop,
            failed,
        })
        .collect();
    tactic_distribution.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tactic.cmp(&b.tactic)));

    latencies.sort_unstable();
    let (min_ms, p50_ms, p95_ms, max_ms, mean_ms) = if latencies.is_empty() {
        (0, 0, 0, 0, 0.0)
    } else {
        let sum: u64 = latencies.iter().sum();
        let mean = ((sum as f64) / (latencies.len() as f64) * 10.0).round() / 10.0;
        (
            latencies[0],
            percentile(&latencies, 50),
            percentile(&latencies, 95),
            *latencies.last().unwrap(),
            mean,
        )
    };

    let surfaced = surface_misses(traces, None);

    RunAnalytics {
        total_steps,
        sessions: sessions_set.into_iter().collect(),
        max_day,
        min_population: if total_steps == 0 { 0 } else { min_pop },
        max_population: max_pop,
        min_water_stock: if total_steps == 0 { 0 } else { min_water },
        min_food_stock: if total_steps == 0 { 0 } else { min_food },
        mean_wellbeing: if total_steps == 0 {
            0.0
        } else {
            ((wellbeing_sum / total_steps as f64) * 10.0).round() / 10.0
        },
        tactic_distribution,
        latency: LatencySummary {
            min_ms,
            p50_ms,
            p95_ms,
            max_ms,
            mean_ms,
            stale_count,
            stale_rate_pct: pct(stale_count, total_steps),
            source_counts,
        },
        total_misses: surfaced.total_misses,
        failure_attribution: surfaced.misses_by_layer,
        surfaced_misses: surfaced.misses,
        timeline,
        frames,
    }
}

/// Render a readable Markdown end-of-run analytics report.
pub fn render_markdown_report(a: &RunAnalytics) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# Timberbot Run Analytics & Replay Summary\n");
    let _ = writeln!(
        md,
        "- **Total decision steps:** {} across {} session(s) (`{}`)",
        a.total_steps,
        a.sessions.len(),
        a.sessions.join("`, `")
    );
    let _ = writeln!(
        md,
        "- **Colony bounds:** max day `{}`, pop `{}`–`{}`, min water `{}`, min food `{}`, mean wellbeing `{:.1}`",
        a.max_day,
        a.min_population,
        a.max_population,
        a.min_water_stock,
        a.min_food_stock,
        a.mean_wellbeing
    );
    let _ = writeln!(
        md,
        "- **Decision latency:** min `{} ms` | p50 `{} ms` | p95 `{} ms` | max `{} ms` | mean `{:.1} ms` | stale `{}` (`{:.1}%`)",
        a.latency.min_ms,
        a.latency.p50_ms,
        a.latency.p95_ms,
        a.latency.max_ms,
        a.latency.mean_ms,
        a.latency.stale_count,
        a.latency.stale_rate_pct
    );
    let _ = writeln!(
        md,
        "- **Decision provenance:** `{:?}`",
        a.latency.source_counts
    );
    let _ = writeln!(
        md,
        "- **Failure-layer attribution:** total misses `{}` (`{:?}`)\n",
        a.total_misses, a.failure_attribution
    );

    let _ = writeln!(md, "## Tactic Distribution\n");
    let _ = writeln!(
        md,
        "| Tactic | Count | Share | Succeeded | NoOp/Blocked | Failed |"
    );
    let _ = writeln!(md, "| --- | ---: | ---: | ---: | ---: | ---: |");
    for t in &a.tactic_distribution {
        let _ = writeln!(
            md,
            "| `{}` | {} | {:.1}% | {} | {} | {} |",
            t.tactic, t.count, t.share_pct, t.succeeded, t.noop, t.failed
        );
    }

    let _ = writeln!(md, "\n## Failure-Layer Attribution\n");
    if a.surfaced_misses.is_empty() {
        let _ = writeln!(
            md,
            "Zero misses across all 4 layers (`StateBug`, `MissingCatalogueTactic`, `PickError`, `PlaybookBug`).\n"
        );
    } else {
        let _ = writeln!(
            md,
            "| Layer | Session#Step | Day | Picked | Suggested | Detail |"
        );
        let _ = writeln!(md, "| --- | --- | ---: | --- | --- | --- |");
        for m in a.surfaced_misses.iter().take(40) {
            let _ = writeln!(
                md,
                "| `{:?}` | `{}#{}` | {} | `{}` | `{}` | {} |",
                m.layer,
                m.source,
                m.step_index,
                m.day,
                m.chosen_tactic.as_str(),
                m.suggested_tactic.as_str(),
                m.detail.replace('|', "\\|")
            );
        }
    }

    let _ = writeln!(md, "\n## Colony Timeline Sample\n");
    let _ = writeln!(
        md,
        "| # | Session | Day | Phase | Pop | Water | Food | WB | River d/c | Tactic | Latency |"
    );
    let _ = writeln!(
        md,
        "| ---: | --- | ---: | --- | ---: | ---: | ---: | ---: | --- | --- | ---: |"
    );
    let stride = (a.timeline.len() / 24).max(1);
    for (i, p) in a.timeline.iter().enumerate() {
        if i % stride == 0 || i + 1 == a.timeline.len() {
            let _ = writeln!(
                md,
                "| {} | `{}` | {} | `{:?}` | {} | {} | {} | {:.1} | {:.2}/{:.2} | `{}` | {} ms |",
                p.index,
                p.session,
                p.day,
                p.weather_phase,
                p.population,
                p.water_stock,
                p.food_stock,
                p.wellbeing_avg,
                p.river_depth,
                p.river_contamination,
                p.tactic,
                p.latency_ms
            );
        }
    }

    md
}

/// Render a self-contained interactive HTML analytics & replay report (inline SVG charts +
/// frame scrubber, zero external CDN dependencies).
pub fn render_html_replay(a: &RunAnalytics) -> String {
    let payload_json = serde_json::to_string(a)
        .unwrap_or_else(|_| "{}".to_string())
        .replace("</", "<\\/");

    let pop_svg = render_svg_line_chart(
        "Population",
        "#2563eb",
        &a.timeline
            .iter()
            .map(|p| p.population as f64)
            .collect::<Vec<_>>(),
    );
    let water_svg = render_svg_line_chart(
        "Water Stock",
        "#0891b2",
        &a.timeline
            .iter()
            .map(|p| p.water_stock as f64)
            .collect::<Vec<_>>(),
    );
    let food_svg = render_svg_line_chart(
        "Food Stock",
        "#16a34a",
        &a.timeline
            .iter()
            .map(|p| p.food_stock as f64)
            .collect::<Vec<_>>(),
    );
    let wb_svg = render_svg_line_chart(
        "Wellbeing Avg",
        "#9333ea",
        &a.timeline
            .iter()
            .map(|p| p.wellbeing_avg)
            .collect::<Vec<_>>(),
    );

    let mut tactic_rows = String::new();
    for t in &a.tactic_distribution {
        let _ = writeln!(
            tactic_rows,
            "<tr><td><code>{}</code></td><td>{}</td><td><div class=\"bar\"><span style=\"width:{:.1}%\"></span></div> {:.1}%</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&t.tactic),
            t.count,
            t.share_pct,
            t.share_pct,
            t.succeeded,
            t.noop,
            t.failed
        );
    }

    let mut attr_rows = String::new();
    for (k, v) in &a.failure_attribution {
        let _ = writeln!(
            attr_rows,
            "<li><code>{k:?}</code>: <strong>{v}</strong></li>"
        );
    }
    if attr_rows.is_empty() {
        attr_rows.push_str("<li><strong>0 misses</strong> across State, Catalogue, Pick, and Playbook layers.</li>");
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Timberbot End-of-Run Analytics &amp; Replay</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; margin: 0; padding: 24px; background: #0f172a; color: #e2e8f0; }}
  h1, h2 {{ margin: 0 0 12px 0; color: #f8fafc; }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px; margin-bottom: 20px; }}
  .card {{ background: #1e293b; border: 1px solid #334155; border-radius: 10px; padding: 14px; }}
  .kpi-label {{ font-size: 12px; color: #94a3b8; text-transform: uppercase; letter-spacing: .04em; }}
  .kpi-val {{ font-size: 24px; font-weight: 700; margin-top: 4px; color: #38bdf8; }}
  .charts {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 14px; margin-bottom: 20px; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
  th, td {{ text-align: left; padding: 6px 8px; border-bottom: 1px solid #334155; }}
  .bar {{ display: inline-block; width: 90px; height: 8px; background: #334155; border-radius: 4px; overflow: hidden; vertical-align: middle; margin-right: 6px; }}
  .bar span {{ display: block; height: 100%; background: #38bdf8; }}
  .controls {{ display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }}
  button {{ background: #2563eb; color: #fff; border: 0; border-radius: 6px; padding: 6px 12px; cursor: pointer; font-weight: 600; }}
  button:hover {{ background: #1d4ed8; }}
  input[type=range] {{ flex: 1; min-width: 200px; }}
  pre {{ background: #020617; border: 1px solid #334155; padding: 12px; border-radius: 8px; overflow-x: auto; font-size: 12px; white-space: pre-wrap; }}
  code {{ color: #7dd3fc; }}
</style>
</head>
<body>
<h1>Timberbot End-of-Run Analytics &amp; Frame Replay</h1>
<div class="grid">
  <div class="card"><div class="kpi-label">Total Steps</div><div class="kpi-val">{total_steps}</div></div>
  <div class="card"><div class="kpi-label">Latency p50 / p95</div><div class="kpi-val">{p50} / {p95} ms</div></div>
  <div class="card"><div class="kpi-label">Stale Decisions</div><div class="kpi-val">{stale} ({stale_pct:.1}%)</div></div>
  <div class="card"><div class="kpi-label">4-Layer Misses</div><div class="kpi-val">{misses}</div></div>
  <div class="card"><div class="kpi-label">Pop Range</div><div class="kpi-val">{min_pop}–{max_pop}</div></div>
  <div class="card"><div class="kpi-label">Mean Wellbeing</div><div class="kpi-val">{mean_wb:.1}</div></div>
</div>

<div class="charts">
  <div class="card">{pop_svg}</div>
  <div class="card">{water_svg}</div>
  <div class="card">{food_svg}</div>
  <div class="card">{wb_svg}</div>
</div>

<div class="card" style="margin-bottom:20px;">
  <h2>Tactic Distribution</h2>
  <table>
    <thead><tr><th>Tactic</th><th>Count</th><th>Share</th><th>Succeeded</th><th>NoOp/Blocked</th><th>Failed</th></tr></thead>
    <tbody>{tactic_rows}</tbody>
  </table>
</div>

<div class="card" style="margin-bottom:20px;">
  <h2>Failure-Layer Attribution</h2>
  <ul>{attr_rows}</ul>
</div>

<div class="card">
  <h2>Interactive Frame Replay</h2>
  <div class="controls">
    <button id="prevBtn">Prev</button>
    <button id="playBtn">Play</button>
    <button id="nextBtn">Next</button>
    <input id="scrubber" type="range" min="0" max="{max_idx}" value="0" />
    <span id="frameCounter">Frame 0 / {total_steps}</span>
  </div>
  <div id="frameMeta" style="margin-bottom:10px;font-size:14px;"></div>
  <pre id="frameDetail"></pre>
</div>

<script id="analytics-data" type="application/json">{payload_json}</script>
<script>
const DATA = JSON.parse(document.getElementById('analytics-data').textContent);
const frames = DATA.frames || [];
let idx = 0;
let timer = null;
const scrubber = document.getElementById('scrubber');
const counter = document.getElementById('frameCounter');
const meta = document.getElementById('frameMeta');
const detail = document.getElementById('frameDetail');

function render(i) {{
  if (!frames.length) {{
    meta.textContent = 'No frames recorded.';
    detail.textContent = '';
    return;
  }}
  idx = Math.max(0, Math.min(frames.length - 1, i));
  scrubber.value = idx;
  const f = frames[idx];
  counter.textContent = `Frame ${{idx + 1}} / ${{frames.length}}`;
  meta.innerHTML = `<strong>${{f.session}}</strong> step #${{f.step_index}} | Day ${{f.day}} (Cycle ${{f.cycle}}, ${{f.weather_phase}}) | Tactic: <code>${{f.chosen_tactic}}</code> (${{f.outcome_status}}) | Latency: ${{f.latency_ms}}ms | Source: <code>${{f.decision_source}}</code>`;
  detail.textContent = [
    `SUMMARY:   ${{f.prompt_summary}}`,
    `RATIONALE: ${{f.rationale}}`,
    `GAUGES:    Pop=${{f.population}} Water=${{f.water_stock}} Food=${{f.food_stock}} WB=${{f.wellbeing_avg}} Logs=${{f.logs}} Planks=${{f.planks}} Gears=${{f.gears}} SP=${{f.science}} River=${{f.river_depth}}d/${{f.river_contamination}}c`,
    `ENABLED:   ${{(f.enabled_tactics || []).join(', ')}}`,
    `STEPS:     ${{(f.steps_executed || []).join(' ; ') || '(no-op)'}}`,
    f.error ? `ERROR:     ${{f.error}}` : null
  ].filter(Boolean).join('\n');
}}

document.getElementById('prevBtn').onclick = () => render(idx - 1);
document.getElementById('nextBtn').onclick = () => render(idx + 1);
scrubber.oninput = (e) => render(Number(e.target.value));
document.getElementById('playBtn').onclick = (e) => {{
  if (timer) {{ clearInterval(timer); timer = null; e.target.textContent = 'Play'; return; }}
  e.target.textContent = 'Pause';
  timer = setInterval(() => {{
    if (idx + 1 >= frames.length) {{ clearInterval(timer); timer = null; e.target.textContent = 'Play'; }}
    else render(idx + 1);
  }}, 350);
}};
render(0);
</script>
</body>
</html>
"#,
        total_steps = a.total_steps,
        p50 = a.latency.p50_ms,
        p95 = a.latency.p95_ms,
        stale = a.latency.stale_count,
        stale_pct = a.latency.stale_rate_pct,
        misses = a.total_misses,
        min_pop = a.min_population,
        max_pop = a.max_population,
        mean_wb = a.mean_wellbeing,
        max_idx = a.frames.len().saturating_sub(1),
    )
}

/// Write `analytics.json`, `report.md`, and `replay.html` into `out_dir`.
pub fn write_analytics_bundle(a: &RunAnalytics, out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("create analytics output dir {}", out_dir.display()))?;
    let json = serde_json::to_string_pretty(a)?;
    let md = render_markdown_report(a);
    let html = render_html_replay(a);
    std::fs::write(out_dir.join("analytics.json"), format!("{json}\n"))?;
    std::fs::write(out_dir.join("report.md"), md)?;
    std::fs::write(out_dir.join("replay.html"), html)?;
    Ok(())
}

fn render_svg_line_chart(title: &str, color: &str, values: &[f64]) -> String {
    let w = 320.0_f64;
    let h = 90.0_f64;
    if values.is_empty() {
        return format!(
            "<div class=\"kpi-label\">{title}</div><svg viewBox=\"0 0 320 90\" width=\"100%\" height=\"90\"></svg>"
        );
    }
    let min_v = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_v - min_v).max(1.0);
    let n = values.len().max(2) as f64;
    let mut pts = String::new();
    for (i, &v) in values.iter().enumerate() {
        let x = (i as f64 / (n - 1.0)) * (w - 16.0) + 8.0;
        let y = h - 10.0 - ((v - min_v) / span) * (h - 22.0);
        let _ = write!(pts, "{x:.1},{y:.1} ");
    }
    format!(
        "<div class=\"kpi-label\">{title} ({min_v:.1} – {max_v:.1})</div>\
         <svg viewBox=\"0 0 320 90\" width=\"100%\" height=\"90\">\
         <polyline fill=\"none\" stroke=\"{color}\" stroke-width=\"2.2\" points=\"{pts}\" />\
         </svg>"
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn pct(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        ((part as f64) / (total as f64) * 1000.0).round() / 10.0
    }
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) * p) / 100;
    sorted[idx]
}
