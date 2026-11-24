---
name: timberborn-control-http-levers
description: "Use when a bot must control or read Timberborn without mods, through the official HTTP API built into v1.0+ (HTTP Lever and HTTP Adapter on localhost:8080), or wants a mod-free fallback or kill switch."
---

# Official HTTP API: HTTP Levers and HTTP Adapters (no mods)

Timberborn 1.0 added two automation buildings that talk to the outside world [1.0 patch notes; timberborn.io; wiki HTTP_API]:
- **HTTP Lever**: costs 2 Gear and 2 Scrap Metal, and **5000 SP**, paid once for *all future settlements*. External calls switch it On or Off or recolor it. Its output feeds any automation input.
- **HTTP Adapter**: costs 2 Gear and 4 Scrap Metal, and **7500 SP**, also paid once globally. It exposes an in-game signal's On/Off state and **calls webhooks** when the signal changes.

This API cannot place buildings or read the economy. It moves only the signals you wire up in game. Treat it as a **thin, reliable control bus**, not a full game API.

## Endpoints (GET or POST, no body. Paths are lower-case and case-sensitive. URL-encode names)
| Purpose | URL |
|---|---|
| List levers | `http://localhost:8080/api/levers` → `[{"name","state","springReturn"}]` |
| One lever | `/api/levers/<NAME>` |
| Switch on / off | `/api/switch-on/<NAME>` · `/api/switch-off/<NAME>` |
| Color | `/api/color/<NAME>/<RRGGBB>` |
| List adapters | `/api/adapters` → `[{"name","state"}]` |
| One adapter | `/api/adapters/<NAME>` |
- The port defaults to **8080** and is set in any HTTP Lever's panel. It applies to all levers and adapters. If 8080 is taken, use a port between 50000 and 65000 [wiki].
- It binds to localhost only. For remote access prefer an SSH tunnel (`ssh -N -L 8080:127.0.0.1:8080 game-user@game-host`), or put an authenticated proxy in front, because the API has no auth.
- In this fork's Rust `timberbot-mcp` server, these endpoints are wrapped as `lever_list`, `lever_get`, `lever_switch_on`, `lever_switch_off`, `lever_set_color`, `adapter_list`, and `adapter_get`.
- Name levers and adapters with letters and digits only, such as `PumpsOn` or `DroughtAlarm`, to avoid encoding mistakes.
- Adapter webhooks: in the adapter panel, set a "call when switched on" URL and a "call when switched off" URL, with method GET or POST. The game makes the call from the host machine, so `localhost:<port>` reaches a listener on the same Mac [DrakeMackley/TimberbornAutomation].
- Python helper: the `timberborn-http` package is mentioned on the wiki but optional and unverified.

## Patterns
1. **Remote kill switch / mode select**: HTTP Lever → Relay (And) → a group of pumps or floodgates. The bot flips modes such as "drought mode" or "badtide mode" with one GET.
2. **Alarm out**: Resource Counter (water < N) → HTTP Adapter → webhook to the bot's local listener. That wakes the planner, giving event-driven control even without the Timberbot mod.
3. **Watchdog**: Chronometer → HTTP Adapter as a heartbeat. If the bot stops receiving it, the game is paused or closed.
4. **Mirror state**: a Memory (latch) fed by sensors → Adapter, so the bot can poll `/api/adapters` as a cheap dashboard of booleans.

## When to use vs Timberbot
- Use the HTTP API as the **fallback** when the Timberbot fork won't load after a game patch, and as the **safety layer** that keeps working without mods.
- Unlocking costs 12,500 SP for both buildings, which is mid-game science. Until then, the official API is unavailable in a fresh profile. Because the unlock is global, do it once.
- Port conflict: Timberbot uses 8085, 8086, and 8091, and the official API uses 8080. They coexist.

## Verify
`curl -s http://localhost:8080/api/levers` should return JSON. If the connection is refused, no HTTP Lever has been built yet, the port was changed, or the game isn't running.
