# Sources and verification status (drafted 2026-11-24)

## Primary
- Steam / GOG patch notes: Timberborn 1.0 (2026-03-12), "One More 1.0 Thing" (automation, 2026-03-02), revised valves (2026-03-06), Update 1.1 (2026-09-03, build 1.1.2.4), main-branch patches 2026-10-06 and 2026-10-20. Steam News API app 1062090, gogdb.org release notes.
- Mechanistry press release for 1.1. Faction preview post "Three Days to Launch" (2026-03-09).
- impuls42/timberbot: README, AGENTS.md, docs/{api-reference, getting-started, websocket-protocol, events, architecture, configuration, macos-testing, timberbot}.md, design/automation-states.md, python/src/timberbot/game_mcp/server.py, user_api/serve.py, openapi.yaml.
- abix-/TimberbornMods: README, docs/api-reference.md, release-notes, Steam Workshop item 3689670088.
- Official HTTP API: timberborn.io API guide. timberborn.wiki.gg/wiki/HTTP_API (via search).
- 0xguy07/timberborn-tools: README, bridge/README.md, docs/mechanics-notes.md, docs/timber-map-format.md (measured on 1.1.2.4).

## Secondary (wiki-derived)
- citybuildertools.com/timberborn: food-and-water guide, buildings/recipes table (wiki infoboxes, 2026-10-02), best-food guide, gears and materials guide, drought calculator.
- timberborn.wiki.gg pages via search snippets: Game Mode, Weather, Irrigation, Monuments, Wind Turbines, Gravity Battery, Power Shafts, Floodgates, Breeding Pod, Rowhouse/Barrack, Wonders, Well-Being.
- r/Timberborn threads on Nomads and Pillars (player reports).
- Low trust, not relied on for numbers: timberborn.org fan articles.

## Open questions / verification status

### Verified from this repository's source
- **macOS `GameManagedDir` path** (former Q1): `timberbot/src/Timberbot.csproj` and `docs/devenv.md` already define the macOS Steam `GameManagedDir` default.
- **No `POST /mcp` on the C# mod** (former Q3): confirmed from `TimberbotHttpServer.cs` and `openapi.yaml`. Python `tbot serve` exposes FastMCP SSE on `127.0.0.1:8091/sse`. This fork also ships a standalone Rust stdio MCP server (`rust/crates/timberbot-mcp`) that needs no Telegram token or ACP agent.
- **Priority enum** (former Q9): `TimberbotWrite.cs` and `openapi.yaml` accept all five levels: `VeryLow`, `Low`, `Normal`, `High`, `VeryHigh`.
- **Automation configure limits** (former Q10): `TimberbotWrite.ConfigureAutomation` supports `DepthSensor`, `ContaminationSensor`, `FlowSensor`, `ResourceCounter`, `PopulationCounter`, `PowerMeter`, `Relay`, `Memory`, `Chronometer`, and `Lever` (`springReturn`, `pinned`, `state`). It does **not** handle `WeatherStation`, `Timer`, or Floodgate On/Off preset heights.
- **Triggering a save over HTTP** (former Q11): confirmed **absent** from `TimberbotHttpServer.cs` and `openapi.yaml`. Only `TimberbotAutoLoad` (main-menu load) and the `autosave` WS event exist; ask the human to save manually before risky batches.

### Still open / unverified (requires a real game install or live playtest)
1. **Latest Timberborn build compatibility**: this repo only pins `MinimumGameVersion` **1.1.2.0** in `timberbot/src/manifest.json` (`docs/game-build.md`). Compatibility with the latest Steam/GOG build (e.g. 1.1.2.4, 1.1.2.7, or newer patches) and the exact Unity runtime version in 1.1 are **unverified**.
2. **Upstream Workshop mod on 1.1**: does the Workshop "Timberbot API" (last updated 2026-04-03) still load on 1.1? (**unverified**)
3. **Housing capacity**: Lodge 3 (0xguy07/wiki) vs ~4 (Timberbot guide). Rowhouse 2 (Timberbot) vs 5 (wiki snippet). Barrack 10. (**unverified**)
4. **Difficulty tables**: temperate, drought, badtide ranges, handicap curve, starting beavers 8 vs 9. (**unverified**)
5. **1.1 wellbeing & endgame costs**: Tier 4 monument awe value and range, Wonder launch costs, new wellbeing maxima after 1.1 attractions. (**unverified**)
6. **Water & power mechanics**: irrigation reach numbers, evaporation rate, Dam spill height (0.65), Aquifer Drill power (400 hp), Steam Engine and Geothermal outputs. (**unverified**)
7. **Breeding & bots**: Breeding Pod inputs and timing, bot biofuel consumption. (**unverified**)
8. **Timer modes & WeatherStation/Floodgate preset automation**: exact in-game `Timer` mode names and whether future game/mod updates add public setters for `ConfigureAutomation`. (**unverified**)
