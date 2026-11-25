# Timberborn skills library

Playbooks that teach a Grok Bot (or any MCP-capable AI assistant) to **play** Timberborn 1.1 (Mechanistry). Every skill is a `SKILL.md` with YAML frontmatter (`name`, a one-line `description` saying when to use it) and a generic, actionable body. Numbers have sources, and uncertain ones are marked *unverified*. When the live game disagrees with a skill, the game wins. Fix the skill.

## Index
| Skill | Use when |
|---|---|
| `timberborn-fundamentals` | always first: core rules, survival math, vocabulary, skill map |
| `timberborn-agent-loop` | driving a live game: pause→observe→plan→act→verify→unpause, managers, triggers, checkpoints |
| `timberborn-control-timberbot-api` | reading or changing the game through the Timberbot mod (HTTP 8085, WS 8086, tbot CLI, tbot serve MCP) |
| `timberborn-control-http-levers` | mod-free control or fallback through the official HTTP Lever/Adapter API (localhost:8080) |
| `timberborn-save-file-analysis` | offline audits and planning from `.timber` saves (0xguy07 save MCP, Timber Dump) |
| `timberborn-folktails` | the faction is Folktails |
| `timberborn-iron-teeth` | the faction is Iron Teeth |
| `timberborn-water-management` | pumps, tanks, dams, levees, floodgates, valves, reservoirs, irrigation, aquifers |
| `timberborn-drought-and-badtide-response` | a hazard is coming, active, or just ended |
| `timberborn-food-and-needs` | food supply, farm sizing, kitchens, basic needs and ailments |
| `timberborn-power` | generators, shafts, batteries, clutches, grid sizing |
| `timberborn-logistics-and-districts` | paths, haulers, builders, storage, distribution, crossings, migration |
| `timberborn-science-and-unlocks` | what to research next, SP income, `not_unlocked` errors |
| `timberborn-build-order-early-game` | a new settlement, day 0 to ~30 |
| `timberborn-mid-late-game-expansion` | scaling after the first hazards: material ladder, bots, districts, vertical, endgame |
| `timberborn-automation-circuits` | in-game sensors, relays, memory, timers, valves, and wiring them through the API |
| `timberborn-map-reading-and-site-selection` | picking a map, difficulty, start, or sites for each function |
| `timberborn-crisis-recovery` | deaths, 0 water or food, contamination, power collapse, deadlocks |
| `timberborn-monuments-and-wellbeing` | raising wellbeing, monuments Tier 1–4, launching the Wonder |
| `timberborn-production-reference` | exact recipe, cost, power, and science numbers for sizing chains |

## How they connect
```
                 fundamentals (always loaded)
                        |
          agent-loop  (runtime cadence; calls everything below)
          /         |                 \
 control-timberbot  control-http-levers  save-file-analysis      <- connectors
          \         |                 /
   map-reading -> build-order-early -> mid-late-expansion          <- phases
          |            |                  |
   faction skill (folktails | iron-teeth) picks prefab names & chains
          |
 water | food-and-needs | power | logistics | science | automation | monuments   <- systems
          |
 drought-and-badtide-response, crisis-recovery                    <- event handlers
          |
 production-reference (numbers for every system)
```
Typical runtime: `agent-loop` wakes on a WS event, then calls `fundamentals` and the faction skill for context. The phase skill picks goals, the system skills turn goals into placements, `control-timberbot-api` runs them, and the event handlers take over during hazards or crises.

## Connectors
A game running on a Mac listens on **that Mac's loopback** (127.0.0.1). A cloud-hosted bot cannot open those ports directly. It needs one of these:
(a) an agent runtime installed on the Mac (tbot, or a small Python manager plus LLM client),
(b) the bot running shell commands on the Mac through a registered-machine tool (`curl http://127.0.0.1:8085/...`, `tbot ...`), with the user approving local tool use, or
(c) an authenticated LAN bind or tunnel (Timberbot `authToken` is mandatory off-loopback. The official API has no auth, so put a proxy with auth in front).
Option (b) is the quickest way to start. Option (a) is best for always-on play with event-driven managers.

| Connector | What it does | Feasibility | Install | How a Grok Bot reaches it |
|---|---|---|---|---|
| **Rust toolkit MCP** (`rust/crates/timberbot-mcp`) | standalone stdio JSON-RPC 2.0 MCP server wrapping Timberbot HTTP (`timberbot_*`), official `:8080` HTTP Levers/Adapters (`lever_*`, `adapter_*`), and read-only `.timber` save inspection (`save_inspect`). No Telegram or ACP required | **Ready (offline + live)**. Builds on Rust stable without game DLLs | `cargo build --release --manifest-path rust/Cargo.toml -p timberbot-mcp` | Register `timberbot-mcp` as a stdio MCP server on the bot machine; point `TBOT_HOST`/`TBOT_PORT` at an SSH tunnel to the game machine (or at `timberbot-mock` for offline dev) |
| **Rust mock server** (`rust/crates/timberbot-mock`) | speaks the exact Timberbot HTTP :8085 + WS :8086 contract (with `authToken` + `409 game_not_ready`) plus optional `:8080` lever API, backed by a deterministic colony sim | **Ready (no game needed)** | `cargo run --manifest-path rust/Cargo.toml -p timberbot-mock -- --ready` | Point `timberbot-mcp`, `timberbot-agent`, or `tbot` at `127.0.0.1:8085` / `:8086` / `:8080` |
| **Timberbot fork** (`impuls42/timberbot` / this fork) HTTP :8085 + WS :8086 | full read/write: place, path, workers, priorities, floodgates, storage, science, distribution, migration, **automation link/configure**. Push WS game events | **Must be built from source** against the game's `Managed/` DLLs. Manifest pins minimum **1.1.2.0**; latest Steam build compatibility is **unverified** (see `docs/game-build.md`) | .NET SDK ≥ 8.0, then `dotnet build` (macOS/Linux/Windows `GameManagedDir` defaults exist in `Timberbot.csproj`; override with `-p:GameManagedDir=...`). Copy `Timberbot.dll`, `manifest.json`, `thumbnail.png` to `Documents/Timberborn/Mods/Timberbot/`. Press **Launch** in the in-game widget on every load | Prefer an SSH tunnel from the bot machine (`docs/remote-access.md`) so the mod stays on `127.0.0.1`. Opt-in non-loopback `listenAddress` requires `authToken` |
| **Timberbot Python MCP** (`tbot serve`) | ~60 MCP tools (`observe`, `place_building`, `set_floodgate`, `link`, …). Every result carries `meta.events`/`advisory` | Feasible. Python 3.10+ | `pipx install 'timberbot[serve]'`. Shipped flow needs `TBOT_TELEGRAM_TOKEN` and an ACP agent CLI. SSE endpoint `http://127.0.0.1:8091/sse`. (There is **no** `POST /mcp` route on the C# mod itself) | Tunnel port 8091 or run locally. For a standalone MCP server without Telegram, use the Rust `timberbot-mcp` binary above |
| **Timberbot upstream** (abix-/TimberbornMods, Workshop 3689670088) | HTTP :8085, webhooks, no automation wiring, no ready gate | **Probably broken on 1.1** (last update 2026-04-03, tagged Update 1.0). Unverified | Steam Workshop subscribe | Same as the fork, if it loads |
| **Official HTTP API** (HTTP Lever / HTTP Adapter) | switch levers on and off, recolor, read lever and adapter states, adapters call webhooks | **Built in**, no mods. Needs 5000 + 7500 SP (unlocked once for all saves) and the buildings placed and wired | Build an HTTP Lever. Set the port in its panel (default 8080) | Via `timberbot-mcp` (`lever_*`, `adapter_*`) over an SSH tunnel to port 8080, or local `curl http://127.0.0.1:8080/api/switch-on/<Name>` |
| **Save-file MCP** (0xguy07/timberborn-tools `bridge/` or Rust `timberbot-save`) | read-only reports from `.timber` saves: overview, stocks, entities, terrain/water stats (0xguy07 adds rich 10-day history reports) | **Excellent**. Pure Rust `save_inspect` is built into `timberbot-mcp`; 0xguy07 stdlib Python 3 verified on 1.1.2.4 | Built in (`timberbot-mcp`), or `git clone https://github.com/0xguy07/timberborn-tools` | Run locally on copied `.timber` saves (no game needed) |
| **Timber Dump** (datvm, Workshop 3517034498) | dumps live specs/blueprints, prefab components, UI, and localization to JSON in `~/Documents/Timberborn/TimberDump` | Feasible (Harmony dependency). 1.1 compatibility **unverified** | Workshop subscribe + Harmony. Click the main-menu button | Copy the dump to the bot's machine and regenerate `timberborn-production-reference` from it |
| Others seen (not evaluated) | `Agroqirax/TimberbornMCP` (MCP at `localhost:8787/mcp` mod framework), 0xguy07 `BeaverBridge` mod (local HTTP), `DrakeMackley/TimberbornAutomation` (adapter webhook rules) | — | — | — |

### Recommended stack
1. **Offline dev & CI**: run `timberbot-mock` + `timberbot-mcp` (and the reference `timberbot-agent` loop) with zero game DLLs.
2. **Live play across two machines**: run Timberborn + the C# mod on the game machine (`127.0.0.1:8085` / `:8086`), open an SSH tunnel from the bot machine (`docs/remote-access.md`), and run `timberbot-mcp` on the bot machine for Grok Bot.
3. **Safety layer**: once unlocked, wire an HTTP Lever "bot kill switch" and HTTP Adapter alarms on `:8080` so control and alerts keep working if the mod breaks after a game patch.

## Status of facts
See `SOURCES.md` for the source list and the open questions. Items marked *unverified* should be checked in game or on timberborn.wiki.gg. The wiki blocked automated fetches during drafting, so its numbers come through search snippets and wiki-derived sites.
