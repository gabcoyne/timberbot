---
name: timberborn-build-order-early-game
description: "Use at the start of a new Timberborn settlement (day 0 through the end of the first hazard, roughly days 0-30) for a step-by-step opening build order for Folktails or Iron Teeth."
---

# Early game build order (day 0 to the first drought recovery)

Start state: a District Center, ~8–9 adults plus some kits (custom default is 8 adults, 4 kits, 130 food, 0 water. Verify on the New Game screen), a few starting goods, and no paths. Games start temperate. On Normal the first drought comes after ~13–17 days and is short because of the handicap.

## Turn 0 (paused)
1. `set_speed 0`. Run `brain`/`summary`: faction, DC coords and entrance, z-level, nearby `treeClusters` and `foodClusters`, weather (`tempDays`).
2. Read the map (see timberborn-map-reading-and-site-selection). Mark the **pump shoreline** (`find_placement` with the pump prefab, take `waterDepth > 0`), the nearest dense **forest**, **berries**, and the **flat moist land** for farms.
3. Plan a path trunk from the DC entrance that **stays off the shoreline**.

## Order (place while paused, then unpause)
| # | Build | Why / notes |
|---|---|---|
| 1 | Path trunk to forest, shore, and berries | nothing works without paths |
| 2 | **Water Pump** (FT) / **Deep Water Pump** (IT) on the deepest reachable shoreline | water first. 12 logs |
| 3 | **Lumberjack Flag** at the densest grown cluster + `mark_trees` (cutting area) | logs pay for everything. Free |
| 4 | **Gatherer Flag** at the biggest berry cluster | food bridge. Free |
| 5 | **Small Tank** ×1–2 (FT 30 each) | drought buffer starts now |
| 6 | **Farmhouse** (FT Efficient Farmhouse, 25 logs / IT Farmhouse, 20 logs) + plant **Carrots** (FT) or **Kohlrabi** (IT) using `find_planting` | 3–4 day crops |
| 7 | **Housing**: Lodge (FT, 3 beds) / Rowhouse (IT) | FT kits come from free beds. IT needs Breeding Pods later |
| 8 | **Inventor** (12 logs) | science starts. Build a second one by ~day 6 |
| 9 | Small Pile / Small Industrial Pile, then a warehouse | stop logs and food lying around |
| 10 | **Lumber Mill** (FT, + Power Wheel 50 hp) / **Industrial Lumber Mill** (IT, + Large Power Wheel) | planks are the first bottleneck |
| 11 | Second Lumberjack + **Forester** (30 SP) | sustainable wood. 1 forester per ~4 lumberjacks. Birch and pine first |
| 12 | Second pump, more tanks, first **Dam** row | drought prep |
| 13 | **Gear Workshop** (100 SP, 120 hp) | gears are the second bottleneck |
| 14 | Campfire, then Shower | cheap wellbeing (social, wet fur) |
Iron Teeth extras: Scavenger Flag on the nearest ruins from day 1, then Metalsmith (150 SP) once planks flow. Breeding Pods only when beds, food, and berries allow.

## Staffing and cadence
- Unpause only after every beaver has a job **or** hauling to do. Recheck `unemployed`/`vacancies` once construction finishes.
- Keep lumberjacks staffed at all times. Keep 1–2 beavers free for hauling early on.
- Check at each `day.start`: foodDays, waterDays, logs. Recheck placement after each `building.finished`.

## Milestones (Normal)
| By | Target |
|---|---|
| Day 3 | pump, lumberjack, gatherer, farm, lodge, inventor all working |
| Day 8 | planks flowing, forester planted, 2 pumps, ≥ 60 water stored, crops harvesting |
| Day of `drought.approaching` | waterDays ≥ max drought + 2, foodDays ≥ +3, outlets closed |
| End of first drought | no deaths. Replant immediately |
| Day ~30 | 12–18 beavers, dam reservoir, Lumber Mill + Forester + Inventors running, gears started |

## Classic opening mistakes
- Paths or houses on the shoreline, which blocks pumps forever.
- Too many houses for the food and water supply (FT population runs away).
- Everyone employed, so no haulers and goods sit still.
- Inventor too late. Forester too late (the forest runs out mid-drought).
- Lumber Mill without power, or farms on dry soil (didn't use `find_planting`).
- Unpausing with idle beavers.
