---
name: timberborn-control-timberbot-api
description: "Use when a bot needs to read or change a running Timberborn game through the Timberbot API mod (HTTP :8085, WebSocket :8086, tbot CLI, or tbot serve MCP). Covers macOS setup, real endpoint and tool names, threading rules, and error handling."
---

# Controlling Timberborn with the Timberbot API mod

**Source repos** (read 2026-11-24):
- `impuls42/timberbot`: the maintained fork. HTTP plus WebSocket plus automation wiring plus MCP. Python package `timberbot` 0.9.0 on PyPI (Python ≥ 3.10). Last push 2026-10-10. **Minimum game version 1.1.2.0** (manifest).
- `abix-/TimberbornMods`: the upstream. HTTP only, plus webhooks, and **no automation wiring endpoints**. Steam Workshop item 3689670088, "Timberbot API", last updated 2026-04-03 and tagged *Update 1.0*. The latest GitHub release is v0.7.1 (2026-04-04). It probably does not load on 1.1 because 1.1 changed `IBlockObjectPlacer.Place`, `Relay`, and `GoodCarrier` [fork README]. **Unverified. Test before relying on it.**

## Architecture
```
Timberborn (Unity) + Timberbot mod
   HTTP  127.0.0.1:8085  /api/*        read + write
   WS    127.0.0.1:8086  /api/ws       state pushes + 68 game events (not ready-gated)
Python `timberbot` (pure network client)
   tbot <command>     one-shot CLI (python-fire), TOON output by default, --json for JSON
   tbot listen        WS event subscriber
   tbot watch         long-running connector that dispatches an agent backend (claude|codex|opencode|custom)
   tbot serve         ACP connector + FastMCP game server on 127.0.0.1:8091 (SSE at /sse) + Telegram bot
```
**Correction to earlier notes:** the C# mod does *not* expose `POST /mcp`. Two MCP options exist outside the game process:
1. **Rust `timberbot-mcp` (this fork, stdio JSON-RPC 2.0):** standalone binary (`rust/crates/timberbot-mcp`) wrapping the Timberbot HTTP API (`timberbot_*`), the official `:8080` HTTP Lever/Adapter API (`lever_*`, `adapter_*`), and read-only `.timber` save inspection (`save_inspect`). Needs no Telegram token or ACP agent.
2. **Python `tbot serve` (FastMCP SSE):** listens on `http://127.0.0.1:8091/sse` with ~60 tools. Requires `pip install 'timberbot[serve]'` and a Telegram token (`TBOT_TELEGRAM_TOKEN`) in the shipped flow [getting-started.md, user_api/serve.py]. Running `tbot serve`'s FastMCP without Telegram would require a small wrapper around `timberbot.game_mcp.server.create_mcp_server(...)` (**unverified**).

## macOS install (fork)
1. Timberborn 1.1 via Steam. Runs natively on Apple Silicon. 1.1 needs macOS 13 or later. Note: `manifest.json` pins minimum game version **1.1.2.0**; compatibility with the latest Steam build is **unverified** (see `docs/game-build.md`).
2. Mod files go in `~/Documents/Timberborn/Mods/Timberbot/` (`Timberbot.dll`, `manifest.json`, `thumbnail.png`). Build from source against your installed game's `Managed/` DLLs:
   - Install .NET SDK ≥ 8.0 (`brew install dotnet`, per `docs/devenv.md`).
   - `dotnet build -p:GameManagedDir="$HOME/Library/Application Support/Steam/steamapps/common/Timberborn/Timberborn.app/Contents/Resources/Data/Managed"` (this macOS default is already present in `timberbot/src/Timberbot.csproj` and `docs/devenv.md`).
   - Copy the build output into the Mods folder. Disable the upstream Workshop "Timberbot API" so you don't get two singletons.
3. `pipx install timberbot && tbot init`. `tbot init` writes the agent prompts to `~/.config/timberbot`. Memory lives at `~/Library/Application Support/timberbot/memory/<settlement>/brain.toon`.
4. Start the game, dismiss the Mod Manager, and load a save. The **Timberbot API widget** appears at the bottom right showing "Not Ready". The player presses **Launch**, which opens the ready gate. The gate closes again on every load.
5. Autoload on macOS: `tbot launch --settlement=X --save=Y` prints Steam launch options (`--tb-settlement X --tb-save Y`) or an `open -a Steam --args -applaunch 1062090 …` one-liner.
6. Optional `settings.json` in the mod folder: `httpPort 8085`, `wsPort 8086`, `wsEnabled`, `listenAddress "127.0.0.1"`, `authToken`, `debugEndpointEnabled`, `maxBodyBytes`. A non-loopback `listenAddress` *requires* `authToken`. Clients then send `Authorization: Bearer <token>` (or `TBOT_AUTH_TOKEN`).
7. Logs: `~/Documents/Timberborn/Mods/Timberbot/timberbot.log` and `~/Library/Logs/Mechanistry/Timberborn/Player.log`.

## Reaching it from a remote bot
The game listens on the **game machine's loopback** (`127.0.0.1`), so a bot on another machine cannot reach `127.0.0.1:8085` directly. Options, safest first (see `docs/remote-access.md`):
1. **Preferred: SSH tunnel** (`ssh -N -L 8085:127.0.0.1:8085 -L 8086:127.0.0.1:8086 game-user@game-host`, plus `-L 8080:127.0.0.1:8080` for HTTP Levers). Keep the mod on `127.0.0.1` with stock `settings.json`, and run `timberbot-mcp` or `timberbot-agent` on the bot machine.
2. Run the agent runtime (`timberbot-agent` or `tbot`) directly on the game machine.
3. Opt-in non-loopback bind: set `listenAddress` (e.g. `0.0.0.0` or LAN IP) AND a non-empty `authToken` in `settings.json` (the mod refuses to start off-loopback without `authToken`). Never expose it without auth.

## HTTP API reference (fork, base `http://127.0.0.1:8085`)
Output is `?format=toon` (default, compact CSV) or `json`. Booleans are **0/1 integers**. List endpoints take `limit` (default 100, 0 = all), `offset`, `name` (substring), and `x`,`y`,`radius` (Manhattan, default 30. Proximity filtering needs both x and y).

**Read**
`GET /api/ping` · `summary` · `time` · `weather` · `power` · `speed` · `workhours` · `notifications` · `alerts` · `population` · `resources` · `districts` · `distribution` · `buildings` · `trees` · `crops` · `gatherables` · `beavers` · `prefabs` · `tree_clusters` · `food_clusters` · `settlement` · `tiles?x1&y1&x2&y2` · `science` · `wellbeing` · `agent/state`

**Write (POST, JSON body)**
| Endpoint | Body |
|---|---|
| `/api/speed` | `{"speed":0-3}` (0 pause, 1 normal, 2 fast, 3 fastest) |
| `/api/workhours` | `{"endHours":1-24}` (default end is 18) |
| `/api/building/place` | `{"prefab","x","y","z","orientation":"south|west|north|east"}` (bottom-left origin. z must equal terrain height) |
| `/api/placement/find` | `{"prefab","x1","y1","x2","y2"}` returns up to 10 placements with `entranceX/Y`, `pathAccess`, `reachable`, `nearPower`, `flooded`, `waterDepth`, `distance` |
| `/api/path/place` | `{"x1","y1","x2","y2","style":"direct|straight","sections":0}` (A* with auto-stairs. Stairs need an unlock) |
| `/api/building/demolish` · `/api/crop/demolish` | `{"id"}` |
| `/api/building/pause` | `{"id","paused":bool}` |
| `/api/building/priority` | `{"id","priority":"VeryLow|Low|Normal|High|VeryHigh","type":"construction|workplace"}` |
| `/api/building/workers` | `{"id","count"}` |
| `/api/building/hauling` | `{"id","prioritized":bool}` |
| `/api/building/recipe` | `{"id","recipe"}` (destroys work in progress) |
| `/api/building/farmhouse` · `/api/building/plantable` | farmhouse action and plantable priority |
| `/api/building/floodgate` | `{"id","height":float}` (clamped 0..max) |
| `/api/building/clutch` | `{"id","engaged":bool}` |
| `/api/building/storage` | `{"id","good":"Water|none","mode":"accept|obtain|supply|empty"}` |
| `/api/building/range` | `{"id"}` returns a work or effect range |
| `/api/cutting/area` | `{"x1","y1","x2","y2","z","marked":bool}` |
| `/api/planting/mark` · `/api/planting/clear` | `{"x1","y1","x2","y2","z","crop"}` |
| `/api/planting/find` | `{"crop","id"}` or an area returns valid irrigated spots |
| `/api/science/unlock` | `{"building":"<prefab>"}` |
| `/api/distribution` | `{"district","good","import":"Auto|Forced","exportThreshold"}` |
| `/api/district/migrate` | `{"from","to","count"}` |
| `/api/automation/link` | `{"sourceId","targetId","input":"a".."h"|"reset"}` |
| `/api/automation/unlink` | `{"id","input"}` |
| `/api/automation/configure` | `{"id","property","value"}` (see timberborn-automation-circuits) |
| `/api/automation/rename` | `{"id","name"}` |
| `/api/ready` | `{"ready":bool}` (the player's Launch/Stop. Agents should not toggle it) |
| `/api/agent/config` · `/api/agent/request` · `/api/agent/message` | connector mode, goal, prompt queue, and in-game log message |
| `/api/debug` · `/api/benchmark` | reflection inspector and benchmark (disabled unless `debugEndpointEnabled`) |

Verified from `TimberbotWrite.cs` and `openapi.yaml`: priority supports all five levels (`VeryLow`, `Low`, `Normal`, `High`, `VeryHigh`), and there is **no** `/api/save` endpoint on the mod.

## MCP tool names
- **Rust `timberbot-mcp` (standalone stdio, no Telegram/ACP):**
  - Mod read: `timberbot_ping`, `timberbot_agent_state`, `timberbot_summary`, `timberbot_weather`, `timberbot_resources`, `timberbot_buildings`, `timberbot_beavers`, `timberbot_tiles`, `timberbot_alerts`, `timberbot_science`, `timberbot_prefabs`.
  - Mod write: `timberbot_set_speed`, `timberbot_set_workhours`, `timberbot_pause_building`, `timberbot_set_workers`, `timberbot_set_priority`, `timberbot_set_floodgate`, `timberbot_set_storage`, `timberbot_find_placement`, `timberbot_place_building`, `timberbot_place_path`, `timberbot_demolish_building`, `timberbot_plant_crop`, `timberbot_mark_trees`, `timberbot_unlock_building`, `timberbot_automation_link`, `timberbot_automation_configure`.
  - Official `:8080` Levers/Adapters: `lever_list`, `lever_get`, `lever_switch_on`, `lever_switch_off`, `lever_set_color`, `adapter_list`, `adapter_get`.
  - Read-only `.timber` save inspection: `save_inspect`.
- **Python `tbot serve` (SSE on `127.0.0.1:8091/sse`, server name `timberbot-game`; every tool takes `cursor`):**
Observe: `observe`, `summary`, `time`, `weather`, `population`, `resources`, `districts`, `buildings`, `trees`, `crops`, `gatherables`, `beavers`, `workhours`, `science`, `wellbeing`, `notifications`, `alerts`, `distribution`, `prefabs`, `power`, `speed`, `tree_clusters`, `food_clusters`, `find_placement`, `find_planting`, `building_range`, `brain`, `list_locations`.
Act: `set_speed`, `pause_building`, `unpause_building`, `set_priority`, `set_haul_priority`, `set_recipe`, `set_farmhouse_action`, `set_plantable_priority`, `set_workers`, `set_floodgate`, `set_workhours`, `set_distribution`, `set_storage`, `set_clutch`, `unlock_building`, `migrate`, `place_building`, `demolish_building`, `demolish_crop`, `mark_trees`, `clear_trees`, `plant_crop`, `clear_planting`, `place_path`, `link`, `unlink`, `configure_automation`, `rename_automation`.
Memory and meta: `set_location`, `remove_location`, `add_task`, `update_task`, `complain`, plus delegate tools when a broker is configured.
Connector permissions auto-approve the `game.*` globs.

## Errors
Format: `{"error":"<code>: <detail>", ...echo fields}`. Codes: `not_found`, `invalid_prefab` (suggests similar names), `invalid_type`, `invalid_param`, `not_unlocked` (+scienceCost, currentPoints), `insufficient_science`, `no_population`, `disabled`, `unknown_endpoint`, `invalid_body`, `operation_failed`, `internal_error`. Others: `401` for a bad bearer token, and **`409 game_not_ready`** when the gate is closed. Under 409 only `/api/agent/*`, `/api/ready`, and `/api/ping` work. Abort and wait for a `state` frame with `ready:true`. Do not retry in a loop.

## Threading and performance (why the rules exist)
- HTTP is parsed on a background thread. **POST handlers run on the Unity main thread** in `DrainRequests()`. Write jobs such as path routing are budgeted at about 1–2 ms a frame. A batch of mutations is applied over several frames, so check each one.
- Reads come from published snapshots refreshed on request ("fresh-on-request"). They are cheap when the game is idle.
- WS clients have a bounded send queue, and slow consumers get dropped. Reconnect with backoff and send a `heartbeat` every ~30 s.

## Usage recipe
```bash
tbot summary; tbot alerts; tbot weather                 # observe
tbot prefabs | grep -i pump                             # NEVER guess prefab names
tbot find_placement --prefab=WaterPump.Folktails --x1=100 --y1=100 --x2=140 --y2=140
tbot place_building --prefab=WaterPump.Folktails --x=.. --y=.. --z=.. --orientation=..
tbot place_path --x1=<DC entrance> --y1=.. --x2=<entranceX> --y2=<entranceY>
tbot buildings --name=WaterPump                         # verify it exists, check construction
```
Rules: always use `find_placement` output for x/y/z/orientation. Path from the DC entrance to `entranceX/entranceY`. Re-check `reachable`. A failed `place` can leave a **ghost entity**, so list the area and demolish strays.

Prefab name traps: Folktails `SmallPile`, `LumberMill`, `EfficientFarmHouse`, `SmallWarehouse` correspond to Iron Teeth `SmallIndustrialPile`, `IndustrialLumberMill`, `FarmHouse`, `MediumWarehouse`. There is no `FarmHouse.Folktails`.
