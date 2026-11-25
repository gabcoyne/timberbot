---
name: timberborn-monuments-and-wellbeing
description: "Use when raising Timberborn wellbeing (the main productivity multiplier), placing attractions/decorations/monuments, planning Tier 1-4 monuments, or building and launching the faction Wonder to complete a map."
---

# Wellbeing, monuments, and wonders

## Why it matters
Wellbeing is the sum of satisfied need bonuses minus penalties. It unlocks tiered bonuses (wiki-sourced via search, verify in game):
| Wellbeing | Work speed | Notes |
|---|---|---|
| 5–6 | +20% | movement +5% |
| 10–11 | +40% | |
| 15–16 | +60% | reaching 15 with FT unlocks Iron Teeth [0xguy07] |
| 20–21 / 25 / 30–31 | +80% / +100% / +120% | |
| 40–41 / 50–51 / 60–61 | +160% / +200% / +240% | |
| 70–71 | **+260%** | movement +70%, life expectancy +120% |
Kits grow faster too. A colony at single-digit wellbeing wastes most of its labour. A real 600-day colony topped out at 14, and 92–94% of adults were *out of range* of the monuments it had built [0xguy07]. **Coverage beats ownership.**

## Need categories (pre-1.1 max ~77 per faction. 1.1 adds buildings, so read `/api/wellbeing`)
- **Folktails**: Basic 5 · Social 11 · Fun 8 · Nutrition 15 · Aesthetics 9 · Knowledge 3 · Awe 26
- **Iron Teeth**: Basic 5 · Social 2 · Fun 17 · Nutrition 17 · Aesthetics 10 · Awe 26
Penalties: Hunger or Thirst unmet −10 each, Contaminated −10, Shelter unmet −3, Injury −2, chipped teeth or bee sting −1, Sleep unmet −1.
1.1 change: wellbeing from Contemplation Spot, Lido, Scratcher, Swimming Pool, Rooftop Terrace, and especially Campfire now decays more slowly.

## Placement rules
- Every attraction or decoration has an **effect radius** (`POST /api/building/range {"id"}`). Put them on routes beavers already walk: home ↔ work ↔ storage.
- Different categories stack. The same building duplicated in one area mostly doesn't.
- Decorative roofs give aesthetics with no path. Rooftop Terrace is a visited building and needs path or stair access.
- Cheapest early points: Campfire (social), Shower (wet fur), Lantern and Shrub (aesthetics), and a second and third food type (nutrition).

## Monuments (Awe) [wiki via search. 1.1 patch notes]
| Tier | Folktails | Iron Teeth | SP | Awe | Range |
|---|---|---|---|---|---|
| 1 | Farmer Monument | Laborer Monument | 1,000 | +3 | ~7 |
| 2 | Brazier of Bonding | Flame of Unity (300 plank, 100 metal parts) | 3,000 | +5 | ~10 |
| 3 | Fountain of Joy | Tribute to Ingenuity | 12,000 | +8 | ~20 |
| 4 (1.1) | Hall of Abundance (700 plank, 300 treated plank, 500 metal block) | Arch of Progress (600 plank, 300 treated plank, 600 metal block) | 15,000 | +8? (search snippet. **Unverified**, may be higher) | ~30 (unverified) |
| Wonder | Earth Recultivator (2,000 gear, 1,500 metal block, 2,000 treated plank; 10 workers) | Earth Repopulator (1,500 gear, 2,000 metal block, 2,000 treated plank; 8 workers) | 20,000 | +10 | map-wide? (unverified) |
Place monuments at the centre of the housing cluster, and check coverage by counting homes inside the range.

## Wonder and map completion
- Wonders are built in 5 visible stages. They must be **activated (launched)** for an extra cost to complete the map.
  - Earth Recultivator launch: ~500 Extract + 500 Paper (wiki via search, unverified).
  - Earth Repopulator launch: ~500 Berries + 500 Treated Planks, plus up to 8 beaver pilots (unverified).
- The first launch shows a stats pop-up, adds a faction badge to the map, and unlocks **Flexible Start** on that map for both factions. You can keep playing after.
- WS events: `wonder.activated`, `wonder.countdown`, `wonder.completed`.
- Launching is a milestone the human may want to watch. **Ask before launching.**

## Wellbeing improvement loop (each temperate season)
1. `GET /api/wellbeing` to find the lowest-filled categories by population share.
2. Pick the cheapest building that raises the weakest category for the most beavers in range.
3. Place it on a high-traffic path near housing and confirm coverage with `building_range`.
4. Fix penalties first: homeless, contaminated, injured.
5. Track `wellbeing.highscore` events and the 1.1 Population statistics tab.
