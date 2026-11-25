# Connect & Play (Two-Machine Timberborn 1.1 Guide)

End-to-end operator guide for running **Timberborn 1.1** on a separate **Gaming Machine** and driving it from a **Grok Bot / Cursor Machine** using the Rust toolkit (`timberbot-mcp`, `timberbot-agent`, `timberbot-drills`) with the full 4-layer inner/outer loop.

```
┌─ Gaming Machine (Windows / macOS / Linux) ──────────┐
│  Timberborn 1.1.2.0+ (Unity)                        │
│    └─ Timberbot C# Mod (compiled against Managed/)  │
│         ├─ HTTP  127.0.0.1:8085  (reads + writes)   │
│         ├─ WS    127.0.0.1:8086  (/api/ws events)   │
│         └─ Lever 127.0.0.1:8080  (optional 1.1 API) │
└──────────────────────────▲──────────────────────────┘
                           │ SSH tunnel (recommended)
                           │ or LAN/Tailscale + Bearer authToken
┌──────────────────────────▼──────────────────────────┐
│ Grok Bot / Cursor Machine (Linux / macOS)           │
│  ├─ timberbot-mcp (stdio MCP -> Grok Bot / Cursor)  │
│  ├─ Inner loop: baseline or fast-model decide()     │
│  │    └─ logs JSONL traces (real-run.jsonl)         │
│  └─ Outer loop (timberbot-drills):                  │
│       ├─ surface-misses / harvest-cases             │
│       ├─ report -> analytics.json, report.md,       │
│       │            replay.html                      │
│       └─ promote -> gates on offline + 10 drills    │
└─────────────────────────────────────────────────────┘
```

> **Build compatibility note:** `timberbot/src/manifest.json` pins `MinimumGameVersion: 1.1.2.0` (`mod` version `0.8.0`). Compatibility with newer Steam/GOG patches beyond `1.1.2.0` is **unverified** until built against that machine's `Managed/` DLLs.

---

## Step 1 — Build & Install the C# Mod on the Gaming Machine

Run these commands on the **Gaming Machine** where Timberborn 1.1 is installed (requires the .NET SDK matching `timberbot/src/Timberbot.csproj`; see [Dev Environment](devenv.md)).

### 1.1 Locate Timberborn's `Managed/` directory

| OS | Default Timberborn `Managed/` path |
| --- | --- |
| **Windows (Steam)** | `C:\Program Files (x86)\Steam\steamapps\common\Timberborn\Timberborn_Data\Managed` (or `C:\Games\Steam\...`) |
| **Linux (Steam)** | `~/.local/share/Steam/steamapps/common/Timberborn/Timberborn_Data/Managed` |
| **macOS (Steam)** | `~/Library/Application Support/Steam/steamapps/common/Timberborn/Timberborn.app/Contents/Resources/Data/Managed` |

### 1.2 Build against that machine's DLLs

```bash
cd timberbot/src
dotnet build -c Release -p:GameManagedDir="/path/to/Timberborn/Timberborn_Data/Managed"
```

The post-build target auto-deploys `Timberbot.dll`, `manifest.json`, `settings.json`, and docs into Timberborn's `Mods/Timberbot` folder. If your Timberborn user data folder is non-standard, copy the build output (`Timberbot.dll`, `manifest.json`, `settings.json`) manually into:

- **Windows:** `%USERPROFILE%\Documents\Timberborn\Mods\Timberbot\`
- **Linux:** `~/.config/unity3d/Mechanistry/Timberborn/Mods/Timberbot/`
- **macOS:** `~/Library/Application Support/Mechanistry/Timberborn/Mods/Timberbot/`

### 1.3 Enable & load a save

1. Launch **Timberborn 1.1**.
2. Open **Mods** on the main menu and enable **Timberbot**. Restart if prompted.
3. Load or start a colony save (Folktails or Iron Teeth).
4. Leave the in-game Timberbot widget **Stopped (`ready=false`)** while you run initial health checks; press **Launch (`ready=true`)** once the tunnel/smoke test is confirmed.

> `ready` resets to `false` on every save load — the player explicitly opts in each session.

---

## Step 2 — Verify `:8085` (HTTP) and `:8086` (WebSocket) on the Gaming Machine

Before touching the network, confirm both mod listeners are up locally on the Gaming Machine:

```bash
# 1. Liveness (always exempt from the ready gate)
curl -sf http://127.0.0.1:8085/api/ping
# -> {"status":"ok",...}

# 2. Ready gate status (false until you click Launch in the in-game widget)
curl -sf http://127.0.0.1:8085/api/ready
# -> {"ready":false}

# 3. Verify ready gate enforcement (MUST return HTTP 409 before Launch)
curl -i http://127.0.0.1:8085/api/summary
# -> HTTP/1.1 409 Conflict  {"error":"game_not_ready",...}

# 4. Click Launch in the in-game Timberbot widget, then re-test summary:
curl -sf http://127.0.0.1:8085/api/summary | head -c 400
```

Verify the WebSocket listener on `:8086` (not ready-gated; stays open across Launch/Stop toggles):

```bash
# Quick HTTP upgrade handshake probe (expects 101 Switching Protocols or WS frame via websocat/python)
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  http://127.0.0.1:8086/api/ws
```

---

## Step 3 — Expose the Mod to the Grok Bot Machine

Choose **one** transport option (full security details in [Remote Access](remote-access.md)).

### Option A (Recommended): SSH Local Port Forwarding

Keep stock `settings.json` on the Gaming Machine (`"listenAddress": "127.0.0.1"`, `"authToken": ""`). Run on the **Grok Bot Machine**:

```bash
ssh -N -o ServerAliveInterval=30 -o ServerAliveCountMax=3 \
  -L 8085:127.0.0.1:8085 \
  -L 8086:127.0.0.1:8086 \
  -L 8080:127.0.0.1:8080 \
  <game-user>@<game-host>
```

From the Grok Bot Machine, the remote game now appears at `127.0.0.1:8085` (HTTP), `127.0.0.1:8086` (WS), and `127.0.0.1:8080` (optional official Timberborn 1.1 HTTP Lever/Adapter API).

### Option B: Direct LAN / Tailscale Bind with Mandatory `authToken`

If SSH is unavailable, edit `Mods/Timberbot/settings.json` on the Gaming Machine:

```json
{
  "httpPort": 8085,
  "wsPort": 8086,
  "listenAddress": "0.0.0.0",
  "authToken": "REPLACE_WITH_64_HEX_RANDOM_SECRET"
}
```

- Generate a token with `openssl rand -hex 32`.
- **Fail-closed guard:** the mod refuses to start if `listenAddress` is non-loopback and `authToken` is empty.
- Pass `Authorization: Bearer <token>` on every HTTP call, and either the same header or `?token=<token>` on WebSocket upgrades (`ws://<game-host>:8086/api/ws?token=<token>`).
- Keep the firewall scoped strictly to your LAN or Tailscale interface — never expose `:8085`/`:8086` to the public internet.

Verify connectivity from the **Grok Bot Machine**:

```bash
# Option A (SSH tunnel):
curl -sf http://127.0.0.1:8085/api/ping

# Option B (LAN / Tailscale with authToken):
curl -sf -H "Authorization: Bearer $TBOT_AUTH_TOKEN" http://<game-host>:8085/api/ping
```

---

## Step 4 — Build & Wire `timberbot-mcp` (stdio) into Grok Bot / Cursor

On the **Grok Bot Machine**, build the Rust binaries once in release mode (requires Rust 1.88+; zero game DLLs needed):

```bash
cd rust
cargo build --release -p timberbot-mcp -p timberbot-agent -p timberbot-drills
```

The standalone binaries are produced at:

- `rust/target/release/timberbot-mcp` (36 stdio MCP tools: mod read/write, official `:8080` Levers/Adapters, read-only `.timber` save inspector)
- `rust/target/release/timberbot-agent` (pause-observe-plan-act-verify-unpause reference bot)
- `rust/target/release/timberbot-drills` (Layer 3/4 offline evals, 10 staged drills, outer-loop miss surfacing & case harvesting, dual promotion gate, JSON/Markdown/HTML analytics & replay)

### Cursor / Grok Bot MCP configuration (`mcp.json`)

Add `timberbot` to your Cursor `~/.cursor/mcp.json` (or Grok Bot MCP config). Remember: the C# mod does **not** serve `/mcp` — `timberbot-mcp` bridges stdio MCP directly to `:8085` / `:8086` / `:8080`:

```json
{
  "mcpServers": {
    "timberbot": {
      "command": "/abs/path/to/timberbot/rust/target/release/timberbot-mcp",
      "args": [
        "--host", "127.0.0.1",
        "--port", "8085",
        "--ws-port", "8086",
        "--save-root", "/abs/path/to/timberbot/scenarios"
      ],
      "env": {
        "TBOT_AUTH_TOKEN": ""
      }
    }
  }
}
```

- For **Option B (LAN / Tailscale)**, replace `"127.0.0.1"` with `"<game-host>"` and set `TBOT_AUTH_TOKEN` to match `settings.json` (all supported env vars: `TBOT_HOST`, `TBOT_PORT`, `TBOT_WS_PORT`, `TBOT_AUTH_TOKEN`, `TIMBERBORN_LEVER_URL`, `TIMBERBORN_SAVE_ROOT`).

---

## Step 5 — Read-Only Smoke Test (Before Any Writes)

Before letting any agent place buildings or pause workplaces, verify read-only visibility from the Grok Bot Machine:

1. Confirm `ready` is `true` (press **Launch** in-game):
   ```bash
   curl -sf http://127.0.0.1:8085/api/ready
   ```
2. Verify core Layer 1 read endpoints return valid JSON without mutating the save:
   ```bash
   for ep in summary weather resources science workhours power; do
     echo "=== /api/$ep ==="
     curl -sf "http://127.0.0.1:8085/api/$ep" | head -c 240
     echo
   done
   ```
3. In Grok Bot / Cursor (via `timberbot-mcp`), call **read-only** MCP tools first:
   - `timberbot_ping`
   - `timberbot_agent_state` (confirms `ready: true`, `mode`, `goal`)
   - `timberbot_summary`
   - `timberbot_weather`
   - `timberbot_resources`
   - `timberbot_buildings` (`limit: 20`)
   - `timberbot_alerts` (and optionally `timberbot_beavers`, `timberbot_science`, `timberbot_tiles`)
4. Pause the game (`speed=0`) or save a backup `.timber` archive before starting autonomous writes.

---

## Step 6 — Run the Inner Loop (Baseline or Fast-Model `decide()`)

Timberbot uses Ryan Perry's 4-layer inner/outer split (see [Four-Layer Outer Loop](four-layer-outer-loop.md)):

- **Layer 1 (`timberbot-state`):** Deterministic `ModSnapshot::fetch` → `ColonyState` (water/food runways, upstream vs intake contamination lag, builder queue ETAs, unreachable sites, disconnected power networks, wellbeing).
- **Layer 2 (`timberbot-tactics`):** Faction-specific 12-tactic catalogue + `decide_with_backend()`:
  - **Deterministic baseline** (`PolicyBackend::DeterministicBaseline`): zero-latency pure Rust selector.
  - **Fast-model / Agent SDK backend** (`PolicyBackend::ModelBacked`): sends `ColonyState::to_prompt_summary()` + `available_catalogue` to any OpenAI-compatible `/v1/chat/completions` endpoint, validates the strict JSON response against enabled tactics, checks latency/staleness (`--llm-stale-ms`, default `1200 ms`), and deterministically falls back to the baseline on any error, hallucination, disabled tactic, or stale reply.
- **Layer 3 (`timberbot-eval`):** Logs every tick as a `DecisionTrace` JSONL row (`real-run.jsonl`).

### Running the reference agent against the live game

```bash
# Dry-run / short burst first (4 decision cycles, pauses during decisions)
./rust/target/release/timberbot-agent \
  --host 127.0.0.1 \
  --port 8085 \
  --ws-port 8086 \
  --cycles 4
```

### Testing a fast-model policy offline before pointing it at a real save

```bash
export TIMBERBOT_LLM_URL="https://api.x.ai/v1/chat/completions"   # or any OpenAI-compatible fast model / Agent SDK gateway
export TIMBERBOT_LLM_MODEL="grok-3-mini"
export TIMBERBOT_LLM_API_KEY="YOUR_KEY"

# Evaluate the fast model on all 18 labeled offline cases (no live game)
./rust/target/release/timberbot-drills offline \
  --cases scenarios/eval-cases.json \
  --llm-url "$TIMBERBOT_LLM_URL" \
  --llm-model "$TIMBERBOT_LLM_MODEL"

# Run the 10 staged mock drills with the fast model and capture JSONL traces
./rust/target/release/timberbot-drills drills \
  --trace-out /tmp/model-drills.jsonl \
  --llm-url "$TIMBERBOT_LLM_URL" \
  --llm-model "$TIMBERBOT_LLM_MODEL"
```

---

## Step 7 — Run Outer-Loop Evals, Analytics & Gated Promotion on Real Traces

After a live session (or staged drill run) writes `real-run.jsonl`, run the outer loop offline on the Grok Bot Machine — **zero live game required**:

### 7.1 Replay traces & surface misses by layer

```bash
# Measure policy drift, state bugs, playbook failures, and stale ticks
./rust/target/release/timberbot-drills replay --traces real-run.jsonl

# Group every failure/drift into Ryan Perry's 4 layers:
#   StateBug | MissingCatalogueTactic | PickError | PlaybookBug
./rust/target/release/timberbot-drills surface-misses \
  --traces real-run.jsonl \
  --cases scenarios/eval-cases.json
```

### 7.2 Harvest labeled eval cases from real failures

```bash
./rust/target/release/timberbot-drills harvest-cases \
  --traces real-run.jsonl \
  --existing-cases scenarios/eval-cases.json \
  --out /tmp/harvested-cases.json
```

Review `/tmp/harvested-cases.json`, confirm each `expected_tactic`, and merge the new cases into `scenarios/eval-cases.json` so that failure class can never silently regress.

### 7.3 Generate end-of-run analytics & interactive HTML replay

```bash
./rust/target/release/timberbot-drills report \
  --traces real-run.jsonl \
  --out-dir reports/real-run
```

This writes three self-contained artifacts into `reports/real-run/`:

- `analytics.json` — full structured metrics (tactic distribution, latency min/p50/p95/max/mean, staleness & fallback rates, population/water/food/wellbeing timeline, failure-layer attribution, and replay frames)
- `report.md` — concise Markdown summary ready to paste into PRs or notes
- `replay.html` — self-contained interactive browser dashboard (inline SVG sparklines + Play/Pause/Scrubber frame inspector, zero external CDNs)

### 7.4 Gate every new tactic or prompt tweak before live play

Save a baseline report before editing tactics, then run `promote` after your change. Promotion succeeds (`exit 0`) **only** when **both** Layer 3 offline evals (100% pass + zero regressions vs baseline) **and** all 10 Layer 4 staged drills pass with zero deaths and zero playbook failures:

```bash
# 1. Capture baseline before editing
./rust/target/release/timberbot-drills offline \
  --cases scenarios/eval-cases.json \
  --out-report /tmp/baseline-eval.json

# 2. Edit tactic/state/playbook/prompt, then run the dual promotion gate
./rust/target/release/timberbot-drills promote \
  --cases scenarios/eval-cases.json \
  --baseline-report /tmp/baseline-eval.json \
  --trace-out /tmp/promoted-traces.jsonl
```
