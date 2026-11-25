# Four-Layer Architecture & Local Grok Bot Outer Loop

Timberbot's Rust toolkit implements Ryan Perry's 4-layer agent architecture (adapted from [Grok Bot Taught My JEV Agent to Play Smash Bros](https://ryanperry.io/post/grok-bot-taught-my-jev-agent-to-play-smash-bros)) for deterministic Timberborn colony management under real time-lag and precedence constraints.

## The Four Layers

| Layer | Crate | Responsibility | Determinism & Contract |
|---|---|---|---|
| **Layer 1: Typed State** | `timberbot-state` | Compiles raw mod HTTP reads (`ModSnapshot`) into a compact `ColonyState`: water/food runways (hours), days until drought/badtide, upstream vs intake badwater lag, builder construction queue & ETAs, blocked prerequisites (power, science, materials, builders, irrigation), workforce shortages, and active multi-day commitments. | **Pure & deterministic**: same `ModSnapshot` (regardless of building list order) always produces byte-for-byte identical `ColonyState`. `validate_invariants()` catches Layer 1 bugs. |
| **Layer 2: Catalogue & Playbooks** | `timberbot-tactics` | Builds a faction-specific `available_catalogue(&ColonyState)` (`Folktails` vs `IronTeeth`) with lead-time estimates and prerequisite blockers. Pure `decide(&ColonyState)` picks **ONE** named `TacticId` (`secure_early_water`, `build_dam_before_drought`, `divert_badtide_floodgates`, `emergency_food_rationing`, `stage_plank_chain`, `unlock_and_plant_forester`, `advance_science_and_queue`, `expand_housing_capped`, `hold_and_monitor`). Deterministic `execute_tactic()` runs sequential HTTP writes step-by-step. | **Zero I/O in `decide()`**: identical function runs in live sessions, staged drills, and offline evals. Enforces lead-time vs hazard checks, power/science/plank prerequisites, and multi-day commitment continuity. |
| **Layer 3: Traces & Offline Eval** | `timberbot-eval` | Logs every tick as JSONL (`DecisionTrace`: `typed_state`, `available_catalogue`, `chosen_tactic`, `playbook_outcome`, `stale`, `latency_ms`). Runs `evaluate_cases()` against `decide()` with **no live game** and classifies every miss into one of 4 root causes (`MissKind`). | Isolates blame before touching code: `StateBug` vs `MissingCatalogueTactic` vs `PickError` vs `PlaybookBug`. |
| **Layer 4: Outer Loop & Drills** | `timberbot-drills` | Staged offline drills against `timberbot-mock` (with builder queues, construction work-hours, material/power/science/irrigation chains, distance penalties, and 1D river wave lag) plus CLI subcommands `offline`, `drills`, and `replay`. | Fast (~1 s) repeatable drills you run before promoting any tactic, state field, or policy edit. |

---

## Miss Classification (`MissKind`)

Every failure in offline eval or trace replay is classified into exactly one layer:

1. **`state_bug` (Layer 1)** — `ColonyState::validate_invariants()` failed or a critical world signal (e.g., upstream badwater, construction ETA, power deficit) was missing or wrong. **Fix in `timberbot-state`**, not in `decide()`.
2. **`missing_catalogue_tactic` (Layer 2 catalogue)** — The expected tactic wasn't enabled (or doesn't exist yet) in `available_catalogue(state)`. **Fix catalogue gating or add a new named `TacticId` + deterministic playbook** in `timberbot-tactics`.
3. **`pick_error` (Layer 2 prompt / goal / priority policy)** — The expected tactic *was* enabled in `available_catalogue(state)`, but `decide()` picked a different tactic. **Fix selection priority, lead-time check, or commitment preemption** in `decide()`.
4. **`playbook_bug` (Layer 2 execution)** — `decide()` chose the right tactic, but `execute_tactic()` returned `OutcomeStatus::Failed` (or left the colony invariant unmet in a drill). **Fix the deterministic step sequence** in `run_playbook()`.

---

## Running the CLI Locally

All commands run offline from `rust/` with **no live Timberborn game**:

```bash
cd rust

# 1. Layer 3 offline eval against labeled cases (no live game, no mock server)
cargo run -p timberbot-drills -- offline --cases ../scenarios/eval-cases.json

# 2. Layer 4 staged drills against timberbot-mock + emit Layer 3 JSONL traces
cargo run -p timberbot-drills -- drills --trace-out /tmp/timberbot-traces.jsonl

# Run a single drill while iterating
cargo run -p timberbot-drills -- drills --filter badtide_unbuilt_floodgate_2d --trace-out /tmp/badtide.jsonl

# 3. Replay recorded JSONL traces against the CURRENT decide() policy
cargo run -p timberbot-drills -- replay --traces /tmp/timberbot-traces.jsonl
```

Pass `--json` to any subcommand for machine-readable reports suitable for automated scripts or Grok Bot tool calls.

---

## How a Local Grok Bot Runs the Outer Loop

A local Grok Bot (or any coding agent with shell access) improves the colony agent **without touching the real game** by looping over traces, offline evals, and staged drills:

```
  1. RUN DRILLS & CAPTURE TRACES
     cargo run -p timberbot-drills -- drills --trace-out /tmp/traces.jsonl --json

  2. AUDIT TRACES & OFFLINE EVAL
     cargo run -p timberbot-drills -- replay --traces /tmp/traces.jsonl --json
     cargo run -p timberbot-drills -- offline --cases ../scenarios/eval-cases.json --json

  3. CLASSIFY ANY FAILURE OR PLATEAU (4-WAY BLAME)
     - StateBug               -> inspect ColonyState::from_snapshot() in timberbot-state
     - MissingCatalogueTactic -> inspect available_catalogue() / add TacticId in timberbot-tactics
     - PickError              -> inspect decide() priority / lead-time check in timberbot-tactics
     - PlaybookBug            -> inspect run_playbook() sequential writes in timberbot-tactics

  4. FREEZE THE REGRESSION AS A LABELED CASE OR DRILL
     - Extract the failing `typed_state` from `/tmp/traces.jsonl`
     - Append a new LabeledCase to `scenarios/eval-cases.json` with `expected_tactic`
     - Confirm `offline` fails with the expected MissKind BEFORE editing code

  5. IMPLEMENT THE MINIMAL LAYER FIX & VERIFY
     - Edit only the blamed layer crate
     - Re-run `offline` (must hit 100%)
     - Re-run `drills` (must pass 100% with 0 deaths & 0 playbook failures)
     - Re-run `replay --traces /tmp/traces.jsonl` to confirm zero unintended decision drift

  6. PROMOTE
     - Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`
     - Commit the new labeled case + drill + layer fix together so the regression can never return
```

### Staged Drills Included

| Drill | Setup | What It Verifies |
|---|---|---|
| `badtide_unbuilt_floodgate_2d` | 2 days before badtide; `Floodgate.Folktails` unfinished behind a `High`-priority Lodge | Picks `divert_badtide_floodgates`, sets Floodgate `VeryHigh` + boosts builders/workday, finishes & raises gate before arrival, keeps intake clean (`< 0.15`), `0` deaths |
| `low_logs_no_forester` | Low logs, 6 remaining trees, `0` planks, `50 SP`, no Forester/LumberMill/PowerWheel | Stages `PowerWheel` before `LumberMill` (`stage_plank_chain`), makes `>= 7` planks, unlocks & builds `Forester.Folktails` (`unlock_and_plant_forester`), plants Pine saplings (`sapling_trees > 0`) |
| `mid_drought_food_crunch` | Active drought, dry river (`0.0`), `< 24h` food runway, paused Farmhouse | Picks `emergency_food_rationing`, unpauses & prioritizes Farmhouse (`VeryHigh`), extends workday to `18h`, harvests grown crops, `0` deaths |
| `power_before_lumber_mill_it` | Iron Teeth unpowered `LumberMill.IronTeeth`, `0` planks, `0` generators | Picks `stage_plank_chain`, builds `LargePowerWheel.IronTeeth`, powers mill, produces `>= 10` planks, `0` deaths |
| `multi_cycle_survival_folktails` | Folktails Normal 30-day multi-cycle run under the 4-layer loop | Completes `>= 1` full temperate + drought cycle, `0` deaths, `0` playbook failures |
| `multi_cycle_survival_ironteeth` | Iron Teeth Normal 30-day multi-cycle run under the 4-layer loop | Completes `>= 1` full temperate + drought cycle, `0` deaths, `0` playbook failures |
