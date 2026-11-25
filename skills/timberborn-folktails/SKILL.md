---
name: timberborn-folktails
description: "Use when the colony's faction is Folktails (nature faction, natural breeding, crops/bakery, water and wind power, bots on biofuel) and the bot needs faction-specific buildings, prefab names, chains, and priorities."
---

# Folktails playbook

Folktails ("Comfort, food, and sturdy wood") are the starter faction and the one the tutorial uses. They breed **naturally in housing** after work hours. They use only **renewable power**: muscle, water, wind, and geothermal. They have the richest cuisine, and Beehives speed up crops. Their transport is **Ziplines**. Bots run on **Biofuel**. The Wonder is the **Earth Recultivator**. Iron Teeth unlock after you reach wellbeing 15 with Folktails [0xguy07 notes, unverified for 1.1].

## Key prefabs (verify with `prefabs | grep`)
| Role | Prefab | Notes |
|---|---|---|
| Housing | `Lodge.Folktails` | 2×2, **3 beds** [0xguy07/wiki]. The Timberbot guide says ~4. Trust `beds` in game |
| | `MiniLodge.Folktails` (1 bed), `DoubleLodge.Folktails`, `TripleLodge.Folktails` (9 beds) | unlocks |
| Farming | `EfficientFarmHouse.Folktails` | 3 workers, 25 logs, 0 SP. There is no `FarmHouse.Folktails` |
| | `AquaticFarmhouse.Folktails` | 150 SP, cattail and spadderdock in shallow water |
| Water | `WaterPump.Folktails` | 12 logs, 1 worker, ~48 water a day. Must straddle the shoreline |
| | `LargeWaterPump.Folktails` | 400 SP, 3 workers, 5 water per 0.33 h (~242 a day) |
| Power | `PowerWheel.Folktails` 50 hp · `WaterWheel.Folktails` ~270 hp per m³/s of current · `WindTurbine.Folktails` (120 SP, 150 peak / ~68 average) · `LargeWindTurbine.Folktails` (1400 SP, 300 peak / ~144 average) | |
| Wood | `LumberjackFlag.Folktails`, `Forester.Folktails` (30 SP), `LumberMill.Folktails` (1 log → 1 plank per 1.3 h, 50 hp, ~12 a day) | |
| Food | `GathererFlag.Folktails`, `Grill.Folktails` (0 SP), `Gristmill.Folktails` (180 SP, 60 hp), `Bakery.Folktails` (160 SP) | |
| Storage | `SmallPile.Folktails`, `SmallWarehouse.Folktails`, `SmallTank.Folktails` (30 water, 15 logs) | |
| Science | `Inventor.Folktails` (12 logs), `Observatory` (1000 SP, 4 workers, 200 hp, **12 SP/h since 1.1**) | |
| Knowledge | `PaperMill` (250 SP), `PrintingPress` (400 SP) → Books (Knowledge +3) | |
| Health | `Herbalist` (300 SP) → Antidotes for contamination. `MedicalBed`, `TeethGrindstone` | |
| Crops boost | Beehive | ~+30% crop growth within ~3 tiles. Not trees. Causes bee stings (−1 wellbeing) |

## Food chains (16 h shift) [citybuildertools]
| Food | Crop (days, yield per tile) | Kitchen | Note |
|---|---|---|---|
| Carrots (raw) | 4 d, 3 | none | best day-1 crop, ~3.6 tiles per beaver |
| Sunflower seeds (raw) | 5 d, 2 | none | weak per tile, second nutrition item |
| Grilled Potatoes | 6 d, 1 | Grill (1 → 4) | one Grill feeds ~46 beavers |
| Bread | Wheat 10 d, 3 | Gristmill (1 wheat → 1 flour per 0.5 h) + Bakery (1 flour + 0.1 log → 5 bread per h) | ~1.8 tiles per beaver. 1 Gristmill feeds 2 Bakeries, and each Bakery feeds ~30 |
| Cattail Crackers | Cattail 8 d, 3 (aquatic) | Gristmill → Bakery (4 per 0.5 h) | best kitchen food, ~48 beavers per Bakery |
| Grilled Spadderdock | 12 d, 3 (aquatic) | Grill | |
| Grilled Chestnuts | Chestnut tree (matures ~23 d, 3 every 8 d) | Grill | |
| Maple Pastries | Wheat flour + Maple syrup | Bakery (3 per 1.5 h) | wellbeing extra (+3) |
Nutrition wellbeing comes from **variety**. Each distinct food adds its own bonus.

## Bots
Bot Part Factory (500 SP) plus Bot Assembler (750 SP, 250 hp). Folktails chassis need Biofuel from the Refinery (400 SP): 2 water + 2 potatoes → 30 biofuel per 3 h is the efficient recipe. Bots consume **1.93 biofuel/day** (decay 58/day, +30 per unit, [wiki Needs](https://timberborn.wiki.gg/wiki/Needs)). Catalyst (maple syrup + extract) and Punch Cards (Printing Press) boost them (0.67/day each). Bots can't work Power Wheels or Inventors.

## Wellbeing ceiling (pre-1.1 table: max 77) [Timberbot guide]
BasicNeeds 5 (Hunger, Thirst, Sleep, Shelter, WetFur via Shower) · Social 11 (Campfire 1, Contemplation Spot 1, Rooftop Terrace 1, Agora 3, Dance Hall 5) · Fun 8 (Detailer 1, Lido 1, Carousel 3, Mud Pit 3) · Nutrition 15 · Aesthetics 9 · Knowledge 3 (Books) · Awe 26 (Farmer Monument 3, Brazier of Bonding 5, Fountain of Joy 8, Earth Recultivator 10).
**1.1 additions**: Sauna (300 SP, 10 log, 10 plank, 5 gear, social), Domed Garden (60 plank, 40 gear, also covers Wet Fur), and the Tier 4 monument **Hall of Abundance** (15,000 SP, 700 plank, 300 treated plank, 500 metal block). Their exact wellbeing points and the new max are **unverified**. Read `/api/wellbeing` in game.

## Folktails-specific priorities
1. Housing drives growth. Keep 2–4 spare beds so kits keep coming, but never build so many that food and water can't keep up.
2. Carrots on day 1, then wheat and bread or potatoes and a Grill by mid-game, then aquatic crops once a stable shallow pool exists.
3. Water wheels need **flowing** water. Put them in a dam spillway or a channel, not in a still lake. Windmills (and the Gravity Battery, 400 SP) cover droughts when rivers stop.
4. Badwater Dome (2000 SP, 100 plank, 200 gear, 100 metal block) caps a badwater source. Folktails only.
5. Ziplines cut long haul times between districts and plateaus.
