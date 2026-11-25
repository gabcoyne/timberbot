# Timberbot

<p align="center">
  <img src="timberbot/src/thumbnail.png" alt="Timberbot — a cybernetic beaver playing Timberborn at a desk">
</p>

> Fork of [impuls42/timberbot](https://github.com/impuls42/timberbot) (itself a fork of [abix-/TimberbornMods](https://github.com/abix-/TimberbornMods)). Adds a standalone Rust agent toolkit (`rust/`, GPL-3.0-or-later), a 4-layer inner/outer play & eval loop, 20 Timberborn 1.1 playbooks (`skills/`), and deterministic offline drills without changing the upstream C# mod HTTP (`:8085`) or WebSocket (`:8086`) contracts.

[Connect & Play (2-Machine)](docs/connect-and-play.md) | [Four-Layer Outer Loop](docs/four-layer-outer-loop.md) | [Mock Fidelity](docs/mock-fidelity.md) | [Getting Started](docs/getting-started.md) | [Remote Access](docs/remote-access.md) | [API Reference](docs/api-reference.md) | [Skills Library](skills/README.md)

## Architecture at a Glance

```
Gaming Machine (Timberborn 1.1.2.0+)          Grok Bot / Dev Machine (zero game DLLs)
  Timberbot C# Mod                              timberbot-mcp     (36 stdio MCP tools)
    ├─ HTTP  :8085  read/write game state  ◀──  timberbot-agent   (pause-observe-plan-act loop)
    ├─ WS    :8086  /api/ws state + events ◀──  timberbot-drills  (offline evals, 10 drills,
    └─ Lever :8080  official 1.1 HTTP API  ◀──                     miss triage, HTML replay, promote)
                                                timberbot-mock    (2D offline colony simulator)
  Python `tbot` CLI / `tbot watch` also works   skills/           (20 Timberborn 1.1 playbooks)
```

- **Pinned game build:** `MinimumGameVersion: 1.1.2.0` (`timberbot/src/manifest.json`). Compatibility with newer Steam/GOG builds is **unverified** until compiled against that machine's `Managed/` DLLs — see [Game Build Compatibility](docs/game-build.md).
- **No `/mcp` on the C# mod:** MCP runs out-of-process via `timberbot-mcp` (Rust stdio) or `tbot serve` (Python FastMCP SSE).

---

## Quickstart Handoff Prompt (Paste into a Fresh Grok Bot)

Copy and paste the block below into a fresh Grok Bot session pointed at this repository:

```markdown
You are operating **Timberbot** from the `gabcoyne/timberbot` repository to play and improve a Timberborn 1.1 colony.

### 1. Ground Yourself in This Repo First
- Read `docs/connect-and-play.md` (prebuilt release ZIP install or maintainer mod packaging, `:8085`/`:8086` verification, SSH tunnel vs. bearer-token bind, and `timberbot-mcp` wiring).
- Read `skills/README.md` and consult relevant playbooks under `skills/timberborn-*/SKILL.md` before making colony decisions.
- Respect `docs/mock-fidelity.md` (distinguish verified Timberborn 1.1 values from mock approximations and unverified items).

### 2. Verify `timberbot-mcp` with a Read-Only Smoke Test (No Writes Yet)
- Confirm the `timberbot-mcp` stdio server is wired (`--host`, `--port 8085`, `--ws-port 8086`, optional `--auth-token` / `TBOT_AUTH_TOKEN`, `--save-root`).
- Run ONLY read-only MCP tools first:
  1. `timberbot_ping` (liveness; gate-exempt)
  2. `timberbot_agent_state` (check `ready`; remember `ready` resets to `false` on every save load until the player clicks **Launch** in the in-game widget)
  3. `timberbot_summary`, `timberbot_weather`, `timberbot_resources`, `timberbot_buildings`, `timberbot_alerts`
- Do NOT call mutating tools (`timberbot_place_building`, `timberbot_pause_building`, `timberbot_set_floodgate`, `timberbot_demolish_building`, etc.) until `ready` is `true` and the read-only snapshot looks sane. Always execute mutating calls sequentially.

### 3. Run the 4-Layer Inner / Outer Improvement Loop
- **Inner loop (play & log traces):** observe Layer 1 typed state (`timberbot-state`), choose one enabled tactic from the 12-tactic Layer 2 catalogue (`timberbot-tactics`), execute its playbook sequentially, and record Layer 3 JSONL traces.
- **Outer loop (offline, zero live game required):** run `timberbot-drills` on traces and labeled cases:
  - `timberbot-drills offline --cases scenarios/eval-cases.json --out-report /tmp/baseline.json`
  - `timberbot-drills replay --traces <trace.jsonl>`
  - `timberbot-drills surface-misses --traces <trace.jsonl> --cases scenarios/eval-cases.json` (triages misses into `StateBug`, `MissingCatalogueTactic`, `PickError`, `PlaybookBug`)
  - `timberbot-drills harvest-cases --traces <trace.jsonl> --existing-cases scenarios/eval-cases.json --out /tmp/harvested.json`
  - `timberbot-drills report --traces <trace.jsonl> --out-dir reports/latest` (writes `analytics.json`, `report.md`, and interactive `replay.html`)
  - **Promotion gate (required before live play after any tactic/state/playbook change):**
    `timberbot-drills promote --cases scenarios/eval-cases.json --baseline-report /tmp/baseline.json --trace-out /tmp/promoted.jsonl`
    Only promote changes that pass **both** offline evals (0 regressions) **and** all 10 staged drills with 0 deaths and 0 playbook failures.
```

---

## Inner Loop vs. Outer Loop: What Works Out of the Box vs. Fast-Model Setup

### What a fresh Grok Bot can already do (no extra model key required)
- **Outer loop:** full trace replay, 4-layer miss surfacing, labeled case harvesting, JSON/Markdown/HTML analytics & frame replay, offline evals (`18` cases), and all `10` staged mock drills via `timberbot-drills`.
- **Live MCP play:** full read/write colony control, `:8080` official HTTP Levers, and read-only `.timber` save inspection through `timberbot-mcp`.
- **Deterministic inner loop (`PolicyBackend::DeterministicBaseline`):** when `TIMBERBOT_LLM_URL` / `--llm-url` is unset, `timberbot-drills` automatically uses the pure-Rust baseline `decide()` policy with zero external network calls.

### Optional: Priming a Fast-Model Inner Loop (`timberbot-tactics/src/llm.rs`)
To run Ryan Perry's fast-model inner loop (e.g., a low-latency fast model or an Agent SDK / Jev-style proxy exposed as an OpenAI-compatible `/v1/chat/completions` endpoint), pass these exact shipped flags / environment variables to `timberbot-drills` (`offline`, `drills`, or `promote`):

| Flag | Env Var | Default | Purpose |
| --- | --- | --- | --- |
| `--llm-url` | `TIMBERBOT_LLM_URL` | *(unset → baseline)* | OpenAI-compatible `/v1/chat/completions` (or direct JSON) endpoint URL |
| `--llm-model` | `TIMBERBOT_LLM_MODEL` | `grok-3-mini` | Model ID sent in the request payload |
| `--llm-api-key` | `TIMBERBOT_LLM_API_KEY` | *(unset)* | Optional `Authorization: Bearer` key for the model endpoint |
| `--llm-timeout-ms` | — | `1500` | HTTP timeout before deterministic fallback |
| `--llm-stale-ms` | — | `1200` | Latency ceiling; slower responses are marked stale and fall back |
| `--llm-goal` | — | *(unset)* | Optional strategic goal appended to the decision prompt |

There are **no** bespoke Cursor- or Jev-specific CLI flags in this repo: expose any fast model or Agent SDK gateway over an OpenAI-compatible HTTP endpoint and configure `TIMBERBOT_LLM_URL`. Every model reply is strictly validated (`{"tactic", "rationale", "commit_days?"}`, unknown fields rejected, tactic must be enabled in `available_catalogue`); on any HTTP error, timeout, stale latency, schema violation, or hallucinated/disabled tactic, `decide_with_backend()` deterministically falls back to baseline `decide()` and logs `DecisionSource::Fallback` in the trace.

---

## Offline Dev & Verification (No Game Required)

```bash
# 1. Build, lint, and run the full Rust test suite (protocol, mock, state, tactics, eval, drills, MCP, agent)
(cd rust && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace)

# 2. Validate all 20 Grok Bot playbooks
python3 scripts/lint_skills.py

# 3. Run the dual promotion gate (18 offline evals + 10 staged drills) and emit JSON/MD/HTML analytics
cargo run --manifest-path rust/Cargo.toml -p timberbot-drills -- promote --cases scenarios/eval-cases.json --trace-out /tmp/drills.jsonl
cargo run --manifest-path rust/Cargo.toml -p timberbot-drills -- report --traces /tmp/drills.jsonl --out-dir /tmp/timberbot-report
```

---

## Documentation

- **[Connect & Play (2-Machine)](docs/connect-and-play.md)** — Install `timberbot-mod-*.zip` from GitHub Releases (or build/package via `scripts/package-mod.{ps1,sh}`), verify `:8085`/`:8086`, SSH tunnel or bearer-token bind, wire `timberbot-mcp`, smoke test, and run inner/outer loops on real traces.
- **[Four-Layer Outer Loop](docs/four-layer-outer-loop.md)** — Layer 1 typed state, Layer 2 faction tactic catalogue & playbooks, Layer 3 trace evals & miss taxonomy, Layer 4 staged drills.
- **[Mock Fidelity](docs/mock-fidelity.md)** — Honest `verified` / `approximated` / `unverified` audit of Timberborn 1.1 mechanics in `timberbot-mock` and `timberbot-save`.
- **[Remote Access](docs/remote-access.md)** — SSH tunnel (recommended) vs. opt-in non-loopback `authToken` setup.
- **[Game Build Compatibility](docs/game-build.md)** — Pinned `1.1.2.0` manifest minimum and verified contract boundaries.
- **[Getting Started](docs/getting-started.md)** & **[Python `tbot` CLI / API Reference](docs/api-reference.md)** — Python client (`pipx install timberbot`), HTTP endpoints, and [WebSocket Protocol](docs/websocket-protocol.md).
- **[Skills Library](skills/README.md)** — 20 Timberborn 1.1 playbooks and source audit ([`skills/SOURCES.md`](skills/SOURCES.md)).
- **[Repo Guide](AGENTS.md)** & **[Developing](docs/developing.md)** — Project layout, C# mod conventions, and release workflow.
- **[SDLC](docs/sdlc.md)** — One open pull request off `main` per owner. An independent verifier posts `PASS` or `PASS+NOTES` on that head before squash-merge. Issue intake is a 10-minute poll.

## Requirements & Credits

- **Requirements:** Timberborn ≥ `1.1.2.0` (latest Steam/GOG build unverified); .NET SDK 8+ (gaming machine C# mod build); Rust ≥ `1.88` (offline toolkit & MCP server); Python 3.10+ (`tbot` CLI & `scripts/lint_skills.py`).
- **Credits:** Forked from [impuls42/timberbot](https://github.com/impuls42/timberbot) and [abix-/TimberbornMods](https://github.com/abix-/TimberbornMods); informed by `mechanistry/timberborn-modding`, `BeaverBuddies`, `datvm/TimberbornMods`, `ihsoft/TimberbornMods`, `CordialGnom/timberborn-unity-modding`, `TimberPrint`, and `toon-format`.
