---
name: timberborn-map-reading-and-site-selection
description: "Use when choosing a Timberborn map, difficulty or start location, or when surveying terrain to pick sites for pumps, dams, farms, forests, housing, industry or a new district."
---

# Map reading and site selection

## Choosing map and difficulty (for a bot learning the game)
- Start on **Normal** on a beginner-friendly map (revised **Waterfalls**, Plains, or a community "Practice Valley" from 0xguy07/timberborn-tools). Move to Hard only after the agent survives 10+ cycles without deaths.
- **Easy** = 40% consumption, 16–19 day temperate, 2–4 day droughts. **Normal** = 13–17 / 5–9. **Hard** = 5–8 / 15–30 (wiki-sourced, verify). **Custom** sets starting adults and kits, food and water, consumption, temperate, drought, and badtide ranges, badtide chance, badtide-free cycles, and the handicap % and cycles.
- Unconventional maps (1.0/1.1): **Oasis** (aquifers, standing water), **Pressure** (underground sources, Unstable Cores), **Spillage** (irrigated enclaves, hazards), **Beaverome**, **Diorama**, **Nomads** (256×256, 1.1, water flow moves between cycles), **Pillars** (192×192, 1.1, fractured upper level over a hostile lower level). Save these for a later curriculum.
- Completing a map with a launched Wonder unlocks **Flexible Start** for that map, so you can choose a start location.

## Survey checklist (paused, turn 0)
Use `brain` (41×41 ANSI map around the DC), `tbot map --x1..y2`, `GET /api/tiles` (terrain height, water depth, moisture, contamination, occupants/ruins), `tree_clusters`, `food_clusters`, and `find_placement` for each key prefab.
1. **Water**: where sources enter and leave, flow direction, the **deepest shoreline tiles** (pump sites), and narrow shallow chokepoints (dam sites). Natural basins make reservoirs.
2. **Hazards**: badwater sources (including *armed* ones: strength 0 now but specified >0 in saves), Badtide Drains, Badwater Seeps, Unstable Cores, Thorns.
3. **Land**: flat moist ground within irrigation reach (~6–16 tiles depending on channel width, and −6 per block of height). Use `find_planting` for real answers.
4. **Wood**: grown tree clusters within ~20 path tiles. Space nearby for a forester belt.
5. **Metal and science**: ruins (scrap), Relics (free SP), Reserve piles, tanks, and warehouses (free starting goods), Geothermal Fields, Aquifers, Ancient Aquifer Drills.
6. **Height**: terraces need stairs (science). Same-z sites are cheaper to reach early.

## Zoning template (around a DC)
```
  [forest + forester belt]      [farms on moist flats, beehives (FT)]
            \                         /
   [industry + power spine] — DC — [housing + wellbeing ring]
            |                         |
  [storage between producers]   [pumps on reserved shoreline, NO paths on shore]
                     [dam / levee reservoir downstream]
```
- Keep housing near the DC and food storage. Wellbeing buildings go on paths beavers already walk.
- Industry goes together on one power network, near log and plank storage.
- Leave 2–3 tile corridors for future paths and power shafts. Don't fill every tile.

## Picking a second-district site
A separate resource region (forest, ruins, river) more than ~40–60 path tiles from the first DC, with its own pumpable water and farmland. Plan the District Crossing location and make sure the two path networks never touch.

## Red flags
- Pump sites with `waterDepth` under ~0.5 (they dry out first in a drought).
- Farms downstream of badwater without a bypass.
- Housing or storage in a floodplain (`find_placement.flooded = 1` now or after a dam is built).
- Building on platforms over a support you might demolish later.
