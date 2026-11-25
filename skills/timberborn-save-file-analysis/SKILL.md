---
name: timberborn-save-file-analysis
description: "Use when a bot needs to analyse a Timberborn colony offline or between sessions by reading .timber save files (ZIP of JSON), for example to audit food/water buffers, labour, power, or to plan without touching the live game."
---

# Offline planning from .timber saves

## Where saves are (macOS)
`~/Documents/Timberborn/Saves/<Settlement>/<save>.timber`. Autosaves land there too. Maps are in `~/Documents/Timberborn/Maps/`. The game install is at `~/Library/Application Support/Steam/steamapps/common/Timberborn/Timberborn.app/…`, and `Contents/Resources/Data/StreamingAssets/Modding/Blueprints.zip` holds the building specs [0xguy07].

## Format (confirmed for 1.0.13 and 1.1.2.4 by 0xguy07/timberborn-tools)
- A save is a ZIP containing `world.json`, `save_metadata.json`, `version.txt`, and a thumbnail. Maps have `map_metadata.json` instead.
- `world.json` = `{GameVersion, Timestamp, Singletons, Entities}`.
  - Singletons include `MapSize`, `TerrainMap`, `WaterMap`/`WaterMapNew`, `SoilMoistureSimulator`, `SoilContaminationSimulator`, `WeatherService`, `DroughtService`, `WeatherDurationService`, `ScienceService`, `BuildingUnlockingService`, `GoodStatistics`, `BeaverStatistics`, `WorkingHoursManager`, `FactionService`, `WellbeingHighscore`, and others [wiki Game_Save_File].
  - Each entity is `{Id (uuid), Template, Components{...}}`. `BlockObject.Coordinates.Z` is the first air voxel. Orientation is `Cw90`/`Cw180`/`Cw270`, or omitted for 0.
- Terrain voxel index = `z*X*Y + y*X + x`. There are 23 vertical slots for every map (MaxGameTerrainHeight 22 + 1).
- Water column index = `level*X*Y + y*X + x`, with values `"depth:contamination:?:floorZ:prevDepth"` and 2 stacked levels.
- **Not stored**: a building's district (derive it from citizens' `Citizen.AssignedDistrict`, or the nearest DC), the power network, and housing capacity (derive it from the most dwellers ever seen: Lodge 3, Mini Lodge 1, Triple Lodge 9).
- Armed badwater sources have `CurrentStrength` 0 with a nonzero `SpecifiedStrength`. They are scripted to activate later. Treat them as future threats.

## Save inspection tools (read-only)
1. **Built-in Rust `timberbot-mcp` (`save_inspect` tool / `timberbot-save` crate):** opens a `.timber` ZIP read-only and returns `GameVersion`, `Timestamp`, `MapSize`, `DayNumber`, `Cycle`, `CycleDay`, `IsHazardous`, template counts, inventory stock totals, and terrain/water/contamination counts.
2. **0xguy07/timberborn-tools (`bridge/`, stdlib Python):** adds 10-day moving averages and deeper history reports:
```sh
git clone https://github.com/0xguy07/timberborn-tools && cd timberborn-tools
python3 bridge/timberborn_mcp.py --selftest | less
# register with any MCP client (stdio):  python3 /path/bridge/timberborn_mcp.py
# TIMBERBORN_DIR overrides ~/Documents/Timberborn
```
Tools: `list_saves`, `overview`, `district_report`, `food_water_report` (rates averaged over N days, default 10), `water_report`, `power_report`, `logistics_report`, `workplace_report`, `construction_report`, `find_buildings`, `good_history`, `raw` (dotted-path walk of the singletons). Each takes optional `save` and `settlement`. The default is the newest save, so after an autosave just ask again.
Exact: population, jobs, homes, per-beaver needs, per-good daily production, consumption, and stock (~500 days of history), workplace productivity, water depth and contamination, floodgate heights, and drought history. Approximate: building district, distances, power (nominal ratings), housing capacity.
Caveats: a new repo (Nov 2026, 0 stars). Read the code before trusting it. It never writes to saves.

## High-value derived metrics
- **Days of buffer** = stock ÷ mean daily consumption over the last ~10 days, per good, per district.
- **Drought exposure** = daily water use × the worst recent drought length, compared with stored water. The game records drought and badtide history, which beats a fixed table.
- **Workplace productivity** = work done ÷ work possible during work hours. Low values mean missing workers, missing inputs, no power, or full output storage. This is the best "sick building" signal.
- **Labour triangle per district**: adults vs filled jobs (idle pool), desired vs assigned workers (unfilled demand), and free beds (the hard cap).

## Workflow
1. After an autosave (or while paused), run `overview`, then `food_water_report`, then `workplace_report`, then `logistics_report`.
2. Turn the findings into a task list, for example "south district sunflower seeds at 1.7 days, so add an import or a farm".
3. Carry out the list on the live game through Timberbot, then confirm after the next autosave.

## Also useful
- **Timber Dump** (Steam Workshop 3517034498, source `datvm/TimberbornMods`, needs Harmony) adds a main-menu button that dumps live, mod-merged specs/blueprints, prefab components, UI, and localization to `~/Documents/Timberborn/TimberDump`. Use it to build an exact cost and recipe table for the current version instead of trusting the wiki. Last confirmed update was Update 7 era (v7.1.2). 1.1 compatibility is unverified.
- 0xguy07 `tools/apidump` prints the public API of the game DLLs, for diffing versions after a patch.
- Map validator: `python3 maptools/validate_map.py map.timber` (about 50 checks) and `measure_arable.py` (irrigated fraction).
