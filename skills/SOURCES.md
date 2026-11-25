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

### Verified from primary public sources (official wiki + recorded repo fixtures)
- **Housing capacity** ([wiki Housing](https://timberborn.wiki.gg/wiki/Housing), `python/tests/fixtures/openapi/prefabs.json`): Folktails Lodge **3** (12 Log), Mini Lodge **1** (50 SP, 5 Log), Double Lodge **6** (150 SP, 20 Log), Triple Lodge **9** (250 SP, 35 Log); Iron Teeth Barrack **10** (40 Log), Large Barrack **16** (400 SP, 70 Log), Rowhouse **5** (180 SP, 20 Log), Large Rowhouse **8** (600 SP, 35 Log).
- **Difficulty tables** ([wiki Game Mode](https://timberborn.wiki.gg/wiki/Game_Mode)): Easy (16–19 d temp / 2–4 d drought, 25% 8c / 1–3 d badtide after 5c, 30% 6c; 40% consumption), Normal (13–17 d / 5–9 d, 38% 5c / 4–8 d after 4c, 15% 5c; 100% consumption), Hard (5–8 d / 15–30 d, 20% 12c / 15–30 d after 3c, 40% 9c; 100% consumption); current table lists 9 adults + 4 children.
- **Wonder launch costs & range** ([wiki Wonders](https://timberborn.wiki.gg/wiki/Wonders), [Earth Recultivator](https://timberborn.wiki.gg/wiki/Earth_Recultivator), [Earth Repopulator](https://timberborn.wiki.gg/wiki/Earth_Repopulator)): `+10` Awe in **40-tile** radius, 4 build stages (25% each); FT launch `500 Paper + 500 Extract`, IT launch `500 Berries + 500 Treated Planks + up to 8 pilots`.
- **Folktails pre-1.1-attractions wellbeing max**: **78** (`python/tests/fixtures/openapi/wellbeing.json`).
- **Water & power mechanics** ([wiki Dam](https://timberborn.wiki.gg/wiki/Dam), [Evaporation](https://timberborn.wiki.gg/wiki/Evaporation), [Irrigation](https://timberborn.wiki.gg/wiki/Irrigation), [Power](https://timberborn.wiki.gg/wiki/Power), [Geothermal Engine](https://timberborn.wiki.gg/wiki/Geothermal_Engine), [Aquifer Drill](https://timberborn.wiki.gg/wiki/Aquifer_Drill)): Dam spill height **0.65**; evaporation `0.04608 × evaporation_value` (~`0.053 m/day` for ≥3-wide pools, `0.200 m/day` for 1-wide); irrigation max **16** tiles (`-6` per +1 elevation, stops at ≥50% pollution); Aquifer Drill **400 hp**, Geothermal Engine **400 hp**, Steam Engine **400 hp**, Numbercruncher **400 hp**.
- **Breeding Pod & bot consumption** ([wiki Breeding Pod](https://timberborn.wiki.gg/wiki/Breeding_Pod), [Needs](https://timberborn.wiki.gg/wiki/Needs)): Breeding Pod consumes **5 Water + 5 Berries over 5 days → 1 kit**; bots consume **1.93 Biofuel/day** (`58/30`).
- Full mechanic-by-mechanic matrix: `docs/mock-fidelity.md`.

### Still open / unverified (requires a real game install or live playtest)
1. **Latest Timberborn build compatibility**: this repo only pins `MinimumGameVersion` **1.1.2.0** in `timberbot/src/manifest.json` (`docs/game-build.md`). Compatibility with the latest Steam/GOG build (e.g. 1.1.2.4, 1.1.2.7, or newer patches) and the exact Unity runtime version in 1.1 are **unverified**.
2. **Upstream Workshop mod on 1.1**: does the Workshop "Timberbot API" (last updated 2026-04-03) still load on 1.1? (**unverified**)
3. **1.1 Tier 4 monuments & new attractions**: exact awe/wellbeing bonus points and effect radii for `Hall of Abundance`, `Arch of Progress`, `Sauna`, `Domed Garden`, `Massager`, and `Dance Pit`, plus the resulting post-1.1 wellbeing maxima. (**unverified**)
4. **Iron Teeth unlock condition in 1.1 & Numbercruncher SP/output**: whether wellbeing 15 still unlocks Iron Teeth in 1.1, and `Numbercruncher` exact unlock SP and science/hr output. (**unverified**)
5. **Timer modes & WeatherStation/Floodgate preset automation**: exact in-game `Timer` mode names and whether future game/mod updates add public setters for `ConfigureAutomation`. (**unverified**)
6. **Timber Dump on 1.1 & headless `tbot serve` wrapper**: whether Workshop Timber Dump loads on 1.1 and whether `tbot serve`'s FastMCP can run standalone without Telegram. (**unverified**)
