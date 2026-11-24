# Offline Scenarios & `.timber` Fixtures

Deterministic mock scenarios and read-only `.timber` test fixtures used by `timberbot-mock`, `timberbot-save`, `timberbot-mcp`, and `timberbot-agent`.

## JSON Mock Scenarios

- `fresh-start.json` — Day 1 Folktails bootstrap colony.
- `incoming-drought.json` — Cycle 1 approaching 4-day drought survival scenario.
- `food-crisis.json` — Paused Farmhouse and critically low food stocks.
- `badtide.json` — Active contaminated river (`0.92`) badtide diversion and recovery.

## `.timber` Archive Fixtures

- `sample.timber` — Minimal synthetic `.timber` save fixture (`GameVersion: 1.1.2.0`) for `timberbot-save` and `timberbot-mcp` (`save_inspect`) tests.
- `practice-valley.timber` — Real Timberborn 1.0/1.1-format `.timber` map archive (`128x128`, 23-layer `TerrainMap.Voxels.Array`, `1644` entities, `GameVersion: 1.0.13.1-b769e88-sm`) sourced from [`0xguy07/timberborn-tools`](https://github.com/0xguy07/timberborn-tools) (`maps/Practice Valley.timber`, MIT License, Copyright (c) 2026 Guy Friley).
