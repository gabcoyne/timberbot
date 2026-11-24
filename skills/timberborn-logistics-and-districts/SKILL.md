---
name: timberborn-logistics-and-districts
description: "Use when goods are not reaching buildings, construction stalls, beavers are idle or homeless, or the colony is outgrowing one District Center in Timberborn: paths, haulers, builders, storage, distribution, district crossings, migration, vertical/transport."
---

# Logistics, labour, and districts

## Paths are everything
- Every building's **entrance tile** must be a path connected to the District Center. `find_placement.reachable` = 1 means connected. `distance` = path cost from the DC entrance. A south-facing DC at (x,y) has its entrance at (x+1, y−1) [Timberbot guide].
- Paths are free. Build them as a tree: DC entrance as the root, then trunks, branches, and entrances. Keep the shoreline free for pumps.
- Path colour turns green to red past ~70 tiles. That's a walking-time warning, not a limit.
- **Builders only build within ~10 tiles of a connected path.** Beyond that the queue silently never moves [0xguy07].
- Height changes need Stairs (unlock), Spiral Stairs (1 log, 4 treated plank, 350 SP, 1×1 per level, stackable), Platforms, Ziplines (FT), or Tubeways (IT). `place_path` auto-places stairs once they're unlocked.

## Labour model
- The three numbers that govern a district: **adults vs filled jobs** (idle pool), **desired vs assigned workers** (unfilled demand), and **free beds** (the hard cap) [0xguy07].
- **Haulers** = Hauling Post workers (10 slots each, 20 log, 10 plank) plus idle beavers. If every adult is employed, goods stop moving. Keep ~10–20% slack or staff Hauling Posts.
- **Builders** = DC workers plus Builders' Huts (4 workers, 400 SP). Raise Builders' Huts to **High priority** while the queue is long. In a labour-short district they lose out to everything else.
- Use workplace priority (`POST /api/building/priority type=workplace`) to decide who gets staffed in a shortage. Use `set_workers` to free beavers from low-value jobs.
- `POST /api/building/hauling {"prioritized":true}` makes haulers serve a starving workshop first.

## Storage
- Three families: piles (logs, planks, metal, dirt), warehouses (food, gears, goods), and tanks (water, badwater, extract, syrup, biofuel). Each building holds **one good at a time**.
- Modes (`POST /api/building/storage`): `accept` (default), `obtain` (actively pull), `supply` (push out), `empty` (drain it so you can repurpose or demolish).
- Put storage between producers and consumers. Workshops far from storage show up in `logistics_report` (save MCP).
- Since 1.1 the inventories of the DC and Builders' Hub no longer count toward top-bar capacity or Resource Counter totals (2026-10 patch).

## When to start a second district
- Work sites cluster more than ~40–60 path tiles from the DC, haul trips dominate, or a separate resource region (ruins, forest, or a second river) opens up.
- Every district needs its own survival chain: water, food, housing, and a lumberjack. Resources do **not** flow between districts on their own.
- **District Crossing** (30 log, 15 plank, 600 SP) links two districts. Up to 10 haulers per side carry goods. The two districts' path networks must not touch.
- Distribution (`POST /api/distribution {"district","good","import":"Auto|Forced","exportThreshold"}`) sets per-good import and export. Export a surplus above a threshold and force imports for what's missing.
- **Migration** (`POST /api/district/migrate {"from","to","count"}`): move only as many adults as the destination has **free beds**. Otherwise they end up homeless. Loop: build housing, staff builders, migrate up to free beds, repeat [0xguy07 case study: 326/378 staffed → 378/378].

## Diagnosis table
| Symptom | Likely cause | Fix |
|---|---|---|
| Building done but unused (small red icon) | entrance tile not on a path | `find_placement`, re-path to `entranceX/Y` |
| Construction frozen | no builders free, no materials, or >10 tiles from path | Builders' Hut High, check planks and logs, extend path |
| Workshop idle with full inputs | output storage full | add storage, or set a consumer to `obtain` |
| Workshop idle with no inputs | haul distance or no haulers | Hauling Post, prioritize hauling, move storage |
| unemployed > 0 and vacancies > 0 | jobs unreachable | paths, district boundary |
| Homeless | beds < adults | build housing **before** migrating or breeding |
