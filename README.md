# Timberbot API

<p align="center">
  <img src="timberbot/src/thumbnail.png" alt="Timberbot — a cybernetic beaver playing Timberborn at a desk">
</p>

**Status: gameplay is stable; AI integration and automation features are under active development**

> Fork of [impuls42/timberbot](https://github.com/impuls42/timberbot), which itself is a modified fork of [abix-/TimberbornMods](https://github.com/abix-/TimberbornMods). Adds a standalone Rust agent toolkit (protocol, async HTTP/WS client, read-only `.timber` save analyzer, deterministic mock server, stdio MCP server, and reference pause-observe-plan-act-verify-unpause agent loop), 20 Grok Bot gameplay playbooks (`skills/`), and offline end-to-end colony scenarios without changing the upstream C# mod HTTP/WS contract. All credit for the original mod goes to [abix-](https://github.com/abix-) and [impuls42](https://github.com/impuls42).

[Getting Started](docs/getting-started.md) | [Game Build Compatibility](docs/game-build.md) | [Remote Access](docs/remote-access.md) | [API Reference](docs/api-reference.md) | [WebSocket Protocol](docs/websocket-protocol.md) | [Skills Library](skills/README.md) | [Upstream](https://github.com/impuls42/timberbot)

C# mod + Python client that lets AI agents read and control a running Timberborn game over HTTP, with a parallel WebSocket channel for push-style state and events.

```
Timberborn (Unity)
  |-- Timberbot API mod
        |-- HTTP server  on :8085  read + write game state
        |-- WS server    on :8086  /api/ws — state pushes + game events

Python (PyPI: `timberbot`)
  |-- `tbot` CLI                     dashboard, agent launcher, raw API client
  |-- `tbot watch`                   long-running WS connector that drives an agent
  |-- `tbot listen`                  WS event subscriber (CLI + scripts)
  |-- `from timberbot import …`      typed Pydantic client for scripts

Rust Toolkit (`rust/`, GPL-3.0-or-later — builds & tests with zero game DLLs)
  |-- `timberbot-protocol`           typed HTTP/WS, official `:8080` Lever/Adapter, `.timber` models
  |-- `timberbot-client`             async HTTP (`:8085`) + WS (`:8086`) + Lever (`:8080`) clients
  |-- `timberbot-save`               read-only `.timber` ZIP (`world.json`) analyzer
  |-- `timberbot-mock`               local mock HTTP :8085 + WS :8086 + Lever :8080 colony sim
  |-- `timberbot-mcp`                standalone stdio MCP server (35 tools; no Telegram/ACP needed)
  |-- `timberbot-agent`              reference pause-observe-plan-act-verify-unpause loop + managers

Playbooks (`skills/`)
  |-- 20 `timberborn-*/SKILL.md` playbooks + `scripts/lint_skills.py`
```

## Grok Bot Toolkit (Offline Dev, MCP & Live Play)

### Verified Game Build Compatibility

- **Pinned minimum build in this repo:** Timberborn **1.1.2.0** (`MinimumGameVersion` in `timberbot/src/manifest.json`, mod version `0.8.0`). See [Game Build Compatibility](docs/game-build.md).
- **Latest Steam / GOG build:** **unverified** (not yet compiled or playtested against the current Steam/GOG binaries in this environment).
- **No `POST /mcp` on the C# mod:** the mod exposes HTTP on `:8085` and WebSocket on `:8086/api/ws` only. MCP runs out-of-process via either `timberbot-mcp` (Rust stdio, standalone) or `tbot serve` (Python FastMCP SSE on `127.0.0.1:8091/sse`).

### Develop & Test Without the Game

Everything in `rust/`, `scenarios/`, and `skills/` builds and runs with no Timberborn install, Steam, or game DLLs:

```bash
# Run all unit tests, HTTP/WS/auth/ready-gate contract tests, MCP tool tests,
# and all 4 E2E scenarios (fresh-start, incoming-drought survival, food-crisis, badtide)
(cd rust && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace)

# Validate all 20 playbooks under skills/
python3 scripts/lint_skills.py

# Run the local mock server with a scenario and drive it with the reference agent
cargo run --manifest-path rust/Cargo.toml -p timberbot-mock -- --scenario scenarios/incoming-drought.json --ready
cargo run --manifest-path rust/Cargo.toml -p timberbot-agent -- --cycles 8

# Or run the standalone stdio MCP server for Grok Bot / any MCP client
cargo run --manifest-path rust/Cargo.toml -p timberbot-mcp -- --save-root scenarios
```

### Pointing the Bot at a Real Game on Another Machine

Keep the C# mod bound to `127.0.0.1` with stock `settings.json` and tunnel from the bot host over SSH (see [Remote Access](docs/remote-access.md)):

```bash
# On the bot machine: forward Timberbot HTTP (:8085), WS (:8086), and optional official HTTP Levers (:8080)
ssh -N \
  -L 8085:127.0.0.1:8085 \
  -L 8086:127.0.0.1:8086 \
  -L 8080:127.0.0.1:8080 \
  <game-user>@<game-host>
```

Then run `timberbot-mcp` or `timberbot-agent` on the bot machine against `127.0.0.1:8085` (and press **Launch** in the in-game Timberbot widget after loading a save). If you instead opt into a non-loopback `listenAddress` in `settings.json`, set a strong `authToken` — the mod refuses to start off-loopback without one.

### Keeping Upstream Merges Clean

All new toolkit code lives in isolated directories (`rust/`, `skills/`, `scenarios/`, `scripts/lint_skills.py`, `docs/game-build.md`, `docs/remote-access.md`, `.github/workflows/toolkit-ci.yml`) and leaves the C# mod and Python client contracts untouched. Pull upstream `impuls42/timberbot` changes directly into `main` without merge friction.

### Verification Status & Open Roadmap

See [`skills/SOURCES.md`](skills/SOURCES.md) and [`docs/game-build.md`](docs/game-build.md) for the full audit trail:

- **Verified from repository source:** `MinimumGameVersion` `1.1.2.0`; HTTP `:8085` + WS `:8086` split with no `/mcp` route on the mod; ready-gate carve-out (`/api/ping`, `/api/ready`, `/api/agent/*`); non-loopback `authToken` guard; 5-level priority enum (`VeryLow`..`VeryHigh`); `POST /api/building/floodgate`; absence of `/api/save`; `ConfigureAutomation` component coverage (`WeatherStation`, `Timer`, and Floodgate preset heights are not wired); macOS/Linux/Windows `GameManagedDir` defaults in `Timberbot.csproj`.
- **Assumed / unverified (needs real game install & playtest):** compatibility of `Timberbot.dll` against the newest Steam/GOG Timberborn build; whether the upstream Workshop mod still loads on 1.1; exact in-game mechanics numbers flagged *unverified* in `skills/` (housing capacities, difficulty handicap tables, irrigation/evaporation rates, 1.1 Wonder costs, Timer mode names); mock simulation rates (intentionally simplified approximations for deterministic CI).

## Quick start

```bash
pipx install timberbot      # or: pip install timberbot
tbot init                   # materialize agent prompts into ~/.config/timberbot
```

```bash
# with Timberborn running + mod loaded
tbot summary                                       # colony snapshot
tbot buildings                                     # list all buildings
tbot beavers                                       # beaver wellbeing + critical needs
tbot map --x1=110 --y1=130 --x2=130 --y2=150               # ASCII map with terrain + blockers
tbot place_building --prefab=Path --x=100 --y=130 --z=2 --orientation=south
tbot place_path --x1=110 --y1=130 --x2=130 --y2=150        # A* pathfinding with auto-stairs
tbot set_speed 3                                   # fast forward (positional)
tbot science                                       # science points + unlockable buildings
tbot distribution                                  # import/export settings per district
tbot link --source-id=42 --target-id=44 --input=a  # wire sensor -> building
tbot configure_automation --id=42 --property=threshold --value=50
tbot brain                                         # live colony state + persistent memory
tbot top                                           # live colony dashboard
tbot                                               # list all commands
```

Auto-launch a save (Linux/Windows) or generate Steam launch args (macOS):

```bash
tbot launch --settlement=MyCastle --save=day5
```

`tbot` itself is a pure network client and does not read the game's Documents
tree. On Linux/Steam Deck the build-time `scripts/deploy.sh` autodiscovers the
Proton/Wine `Documents/Timberborn/Mods/Timberbot/` folder when copying the
freshly built DLL; set `TBOT_DOCUMENTS_DIR` for non-`steamuser` Wine prefixes.

Or use raw HTTP. no Python needed:

```bash
curl http://127.0.0.1:8085/api/summary
curl -X POST http://127.0.0.1:8085/api/speed -d '{"speed": 3}'
```

Live state + game events over the WebSocket:

```bash
tbot listen --pretty                                # subscribe to ws://127.0.0.1:8086/api/ws
tbot watch --backend claude                         # long-running agent connector
```

## Features

- **WebSocket push channel**. State changes and game events stream over `ws://<host>:8086/api/ws` as `{type, payload}` JSON frames. See [WebSocket Protocol](docs/websocket-protocol.md).
- **Ready gate**. Player presses **Launch** in the in-game widget to authorize agent activity; until then all `/api/*` calls (read + write) return `409 game_not_ready` except `/api/agent/*`, `/api/ready`, `/api/ping`.
- **`tbot watch` connector**. Long-running Python process that connects over WS, heartbeats, and dispatches agent runs (request or autonomous mode).
- **A* pathfinding**. `place_path` routes around obstacles, water, and ruins with auto-stairs.
- **Automation wiring**. Link/unlink sensors, relays, memory cells to any pausable building, and configure thresholds, modes, logic over HTTP.
- **Fresh-on-request reads**. No stale data, zero cost when idle.
- **Blocker tracking**. Ruins and editor objects visible in `/api/tiles` and placement errors.
- **Write job system**. Budgeted frame execution, no spikes.
- **Debug endpoint**. Reflection inspector with chaining and validation.
- **Bearer auth**. Optional `authToken` setting; mandatory when `listenAddress` is non-loopback.
- **Safe by default**. Binds to `127.0.0.1`, refuses to start with a non-loopback listenAddress unless `authToken` is set, caps request body size.
- **Zero-alloc hot path**. No garbage collection pressure on read endpoints.

## Docs

- [Getting Started](docs/getting-started.md). install, first steps, examples
- [Game Build Compatibility](docs/game-build.md). pinned game version (`1.1.2.0`), unverified latest-build notice, checked contract gaps
- [Remote Access](docs/remote-access.md). SSH tunnel (preferred) vs opt-in token-protected LAN bind
- [Skills Library](skills/README.md). 20 Timberborn 1.1 Grok Bot playbooks and source audit ([`skills/SOURCES.md`](skills/SOURCES.md))
- [API Reference](docs/api-reference.md). all HTTP endpoints
- [WebSocket Protocol](docs/websocket-protocol.md). `/api/ws` frame envelope, message types, reconnect
- [Events](docs/events.md). consume the game-event stream with `tbot listen` or a custom WS client
- [Timberbot Guide](docs/timberbot.md). AI guide for agents playing Timberborn
- [Architecture](docs/architecture.md). internals, thread model, read/write pipeline, WS broadcaster
- [Automation Plan](design/automation-plan.md). decompiled wiring API and `/api/automation/*` design
- [Agent Prompts](python/src/timberbot/agent_prompts/). drop-in gameplay prompts (`timberbot`, `scout`, `wirer`, `auditor`, `connector-mode`). Materialize editable copies into your user config dir with `tbot init`. The development-agent prompt for working on this codebase lives separately at [`agents/beaver-developer.md`](agents/beaver-developer.md).
- [Repo Guide](AGENTS.md). project layout, build commands, conventions
- [Developing](docs/developing.md). build from source, add endpoints, cutting a GitHub release

## Settings

Drop a `settings.json` in your mod folder (`Documents/Timberborn/Mods/Timberbot/`):

```json
{
  "httpPort": 8085,
  "wsPort": 8086,
  "wsEnabled": true,
  "listenAddress": "127.0.0.1",
  "authToken": "",
  "debugEndpointEnabled": false,
  "maxBodyBytes": 1048576
}
```

All fields are optional. Missing keys use defaults. The server binds to
`127.0.0.1` by default; set `listenAddress` to `+` or `0.0.0.0` to accept LAN
connections — but then `authToken` becomes mandatory (the mod refuses to start
otherwise).

Older settings keys are ignored with a one-line deprecation warning at load:

- Agent-spawn keys (`terminal`, `pythonCommand`, `agentBinary`, `agentModel`, `agentEffort`, `agentCommandTemplate`, `agentAllowlistEnabled`, `agentAllowedBinaries`) — manage agent-side defaults via `~/.config/timberbot/config.toml` instead.
- Webhook keys (`webhooksEnabled`, `webhookBatchMs`, `webhookCircuitBreaker`, `webhookMaxPendingEvents`, `webhookValidateUrls`) — outbound HTTP webhooks are gone; subscribe to the WS event stream instead (`tbot listen` or a custom WS client).

## Requirements

- Timberborn ≥ 1.1.2.0 (Steam; latest Steam/GOG build compatibility is unverified — see [Game Build Compatibility](docs/game-build.md))
- .NET SDK 8+ (to build the C# mod against your local `Managed/` DLLs)
- Rust stable ≥ 1.88 (for the offline `rust/` toolkit: `timberbot-mock`, `timberbot-mcp`, `timberbot-agent`)
- Python 3.10+ (for the `tbot` CLI and `scripts/lint_skills.py`; install via `pipx install timberbot`)

## Credits

Forked from:

- [abix-/TimberbornMods](https://github.com/abix-/TimberbornMods). upstream Timberbot mod. this repo is a modified fork with added HTTP API, automation wiring, WebSocket event stream, and AI-agent tooling

Learned from these Timberborn modding projects:

- [mechanistry/timberborn-modding](https://github.com/mechanistry/timberborn-modding). official modding tools, wiki, and examples
- [thomaswp/BeaverBuddies](https://github.com/thomaswp/BeaverBuddies). `BlockObjectPlacerService.Place()` for building placement, `TemplateInstantiator` + `MarkAsPreviewAndInitialize` + `IsValid()` for game-native placement validation, `BuildingUnlockingService.Unlock()` for science, `WorkingHoursManager` for work schedules
- [datvm/TimberbornMods](https://github.com/datvm/TimberbornMods). `TreeCuttingArea.AddCoordinates()` for tree marking, `IAlertFragment` patterns for building alerts
- [ihsoft/TimberbornMods](https://github.com/ihsoft/TimberbornMods). `Inventories.AllInventories` for building inventory, `BuildingUnlockingService.Unlocked()` for science checks
- [CordialGnom/timberborn-unity-modding](https://github.com/CordialGnom/timberborn-unity-modding). `PlantingService.SetPlantingCoordinates()` for crop planting, `PlantingAreaValidator.CanPlant()` for planting validation
- [Timberborn-KyP-Mods/TimberPrint](https://github.com/Timberborn-KyP-Mods/TimberPrint). `PreviewFactory` + `BlockValidator` patterns for placement validation
- [toon-format/toon](https://github.com/toon-format/toon). Token-Oriented Object Notation for compact AI output
