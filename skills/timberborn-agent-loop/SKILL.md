---
name: timberborn-agent-loop
description: "Use when a bot is actively driving a live Timberborn game and needs the pause-observe-plan-act-verify-unpause cadence, the event-driven manager design, checkpoints, and the rules for when to stop and ask the human."
---

# The Timberborn agent loop

An LLM is too slow and too expensive to make every tick-level decision. Split the work in two:
- **Deterministic managers**: small loops (for example, the reference Rust `rust/crates/timberbot-agent` binary or Python scripts) that hold invariants, such as "keep pumps staffed" or "close the floodgate when a badtide starts".
- **The LLM planner**: runs at decision points. It sets goals, picks sites, chooses unlocks, and handles anything new or unexpected.

## The cycle (one "turn")
```
PAUSE   -> set_speed 0 (POST /api/speed {"speed":0})
OBSERVE -> brain / summary, alerts, weather, resources, wellbeing (reads may run in parallel)
PLAN    -> write a short ordered task list with success checks (persist it: add_task / brain.toon)
ACT     -> execute mutations ONE AT A TIME; check each response's error prefix before the next
VERIFY  -> re-read the affected entities (buildings?id=, find_placement reachable, alerts)
UNPAUSE -> set_speed 1-3 only when every beaver has work and nothing is CRITICAL
RUN     -> let time pass until the next trigger (event, timer, or threshold)
```
Things that still work while paused: placing buildings, paths, priorities, worker counts, recipes, and automation wiring [Timberbot guide]. Nothing gets produced while paused, so do not stay paused longer than the planning takes.

## Triggers that should wake the planner
Subscribe to the Timberbot WebSocket (`ws://127.0.0.1:8086/api/ws`, frames `{type, payload}`, event names in `payload.event`). Treat these events as wake-ups:
- `drought.approaching`, `drought.start`, `drought.end`, `cycle.start`, `weather.selected`
- `beaver.died` (from `TimberbotEntityRegistry`), `character.killed` (from `TimberbotEvents`), `game.over`, `contamination.changed`, `status.alert`
- `building.finished` (plan the next step), `building.unlocked`, `wonder.*`
- `power.network.*` (generators changed), `autosave` (good checkpoint moment)
- Timer fallback: once per in-game day (`day.start`) for a cheap health check, and a full replan every cycle.

When using `tbot serve` MCP, every tool result has `meta.events` and `meta.advisory` (`normal`, `attention`, `urgent`, `halt`). Pass `meta.cursor.high_water` back as `cursor` on the next call. On `urgent`, stop the current plan and fix the problem. On `halt`, end the session [timberbot game_mcp/server.py].

## Health check thresholds (start points. Tune them per difficulty)
| Signal | Green | Act | Critical (pause and replan) |
|---|---|---|---|
| waterDays (stock ÷ daily use) | ≥ longest expected drought + 2 | < drought length + 2 | ≤ 1 |
| foodDays | ≥ drought length + 3 | < drought length + 3 | ≤ 1 |
| logDays / plankDays | ≥ 3 | < 3 | 0 with a construction queue |
| unemployed > 0 while vacancies > 0 | — | check paths and reachability | — |
| homeless > 0 | — | build housing | homeless during drought |
| alerts: unstaffed, unpowered, unreachable | 0 | fix within the turn | — |
| wellbeing average | rising | flat for 2 cycles | falling, or anyone "critical" |
`brain` and `summary` already report `foodDays`, `waterDays`, `logDays`, `plankDays`, `gearDays`, `beds`, `homeless`, `unemployed`, `wellbeing`, `miserable`, and `critical` [abix api-reference].

## Manager catalogue (deterministic, no LLM)
| Manager | Invariant | Mechanism |
|---|---|---|
| WaterKeeper | tanks are full before every hazard. Pumps pause during badtide | resources poll + `set_workers` / `pause_building`. Better still: in-game Weather Station or Contamination Sensor wiring |
| FoodKeeper | foodDays ≥ target, replant after a hazard | `crops`, `plant_crop`, farmhouse priority |
| WoodKeeper | ≥ 1 lumberjack per ~15 beavers, foresters replanting | `mark_trees`, `set_workers` |
| LaborBalancer | haulers exist: keep 10–20% of adults unassigned or at Hauling Posts | `set_workers`, priorities |
| BuilderBooster | builders' huts at High priority while the queue > 0 | `set_priority type=workplace` |
| HazardSwitch | close or open floodgates on `drought.start` / `drought.end` | `set_floodgate`, or wire it in game |
| Checkpointer | named save before any risky batch | game autosave plus a manual save. Verified from `TimberbotHttpServer.cs` and `openapi.yaml`: this mod has no `/api/save` endpoint, so ask the human |
Prefer **in-game automation** (sensors and relays) for anything that must react within an in-game hour. In-game automation keeps working when the bot is offline. Managers are for policy, not reflexes.

## Checkpoints and rollback
- Before a risky batch (demolition, dam rework, mass migration), ask the human to save, or rely on the newest autosave. Record the save name in persistent memory.
- Log every mutation with its request and response. A failed placement can leave a "ghost" entity [api-reference known issue]. Re-list the area with `buildings x,y,radius` and demolish strays.
- The human may be playing at the same time. Re-read state before acting on an old plan.

## Hard rules
1. Never run mutations in parallel. Each one changes the map that the next one depends on.
2. Parse the error prefix (`not_found`, `invalid_prefab`, `invalid_type`, `invalid_param`, `not_unlocked`, `insufficient_science`, `no_population`, `disabled`, `operation_failed`) and branch on it. Do not retry the same failing call blindly.
3. `409 game_not_ready` means the player pressed Stop or just loaded a save. Abort cleanly and wait. Do not loop.
4. Do not unpause into a known shortage. Fix it while paused first.
5. Changing a recipe destroys work in progress. Do not churn recipes.
6. Escalate to the human when: a save or load is needed, the ready gate is closed, a Wonder launch or other irreversible milestone is about to happen, or the same task fails twice for the same reason.

## Turn report template
```
day N (cycle C, temperate|drought|badtide, Xd left) | speed S
pop A adults / K kits / B bots | beds used/total | unemployed U
water Wd | food Fd | logs Ld | planks Pd | gears Gd
wellbeing avg/max | alerts: ...
did: <mutations, with result>
next trigger: <event or day>
```
