---
name: timberborn-water-management
description: "Use when designing or adjusting water infrastructure in Timberborn: pumps, tanks, dams, levees, floodgates, fill/throttling valves, reservoirs, irrigation, aquifers, and canals, including sizing storage for the next hazard."
---

# Water management

Water is three things at once: **drink** (pumps and tanks), **irrigation** (moist soil for crops and trees), and **power** (flowing water turns wheels). During a drought sources stop. During a badtide they emit badwater. The skill is about holding clean water back and controlling where it goes.

## Structures (1.1 values. SP = science to unlock)
| Structure | Cost / SP | Behaviour |
|---|---|---|
| Dam | 20 logs, 0 SP | holds water to **0.65** of a block, and overflow spills ([wiki Dam](https://timberborn.wiki.gg/wiki/Dam)) |
| Levee | 12 logs, 120 SP | watertight 1×1 wall. Stackable, beavers walk on top |
| Floodgate / Double / Triple | 10 log + 5 plank, 150 SP / 20+10, 250 SP / 30+15, 500 SP | adjustable spill height up to 1 / 2 / 3 blocks. When automated, it switches between a height for On and a height for Off. No walking on top |
| Fill Valve | 5 plank, 5 metal block, 300 SP | automatically keeps the downstream water level (the old Sluice logic). Add a Contamination Sensor for badwater rejection |
| Throttling Valve | 5 treated plank, 5 metal block, 500 SP | limits flow rate (0 = closed). With automation it has separate On and Off rates plus an adapt speed |
| Gate | 6 plank, 4 gear, 4 scrap, 200 SP | blocks *beaver* traffic, for example closing a path through a river before a badtide |
| Airlock (1.1) | 15 log, 8 gear, 4 metal block, 300 SP | a tunnel built into a levee |
| Impermeable Power Shaft (1.1) | 12 log, 6 gear, 10 metal block, 300 SP | runs power through a levee wall |
| (Deep) Mechanical Pump | 50 gear, 15 treated plank, 2000 SP, 500 hp (1.1 rebalance) | lifts water |
| Compact Mechanical Pump (1.1) | 50 gear, 25 treated plank, 25 metal block, 4000 SP, 700 hp | pumps straight through a wall. Stackable, so it enables pressurized pipes |
| Discharge (was Fluid Dump) | 10 log, 10 plank, 250 SP | releases 3 stored water or badwater per hour as a flowing source. Use it to irrigate from tanks during drought |
| Aquifer Drill | 40 plank, 25 gear, 15 metal block, 400 SP, **400 hp** ([wiki Aquifer Drill](https://timberborn.wiki.gg/wiki/Aquifer_Drill)) | built over an Aquifer, produces water. Ancient Aquifer Drills come pre-placed on some maps and only need power |
| Tanks | Small 30 (15 logs) · Medium 300 (30 plank, 20 gear, 2×2) · Large 1200 (80 plank, 60 gear, 30 metal block, 3×3) | |
Map objects: **Water Seep** (fills only to 0.8 height, gives shallow wetland water that dries fast), Badwater Seep, Badtide Drain (badwater only during badtides), Natural Dam (holds half a tile).

## Sizing rules
- Drinking: **2.12 water per beaver per day** (Normal/Hard). A basic pump gives ~48 a day on a 16 h shift, so 1 pump per ~20 beavers just for drinking. The Timberbot guide's rough rule is 2 pumps per 15 beavers, and 3 before a drought, which leaves surplus to fill tanks.
- Storage target = beavers × 2.12 × (longest expected drought + 2-day buffer). Example: 50 beavers on Normal with a 9-day drought gives ~955 water, so 4 Medium or 1 Large tank [citybuildertools].
- Refill check: pumps must produce `storage ÷ shortest temperate length + daily drink`. Example: 120 beavers storing 2,800 water over a 13-day temperate season need ~470 water a day of pump output.
- Recheck after any change to working hours, because pumps only run during work hours.
- **Reservoirs beat tanks at scale.** A dammed basin holds thousands of units for a few logs, and its water also irrigates and feeds pumps. Tanks are for when there is no geography to use. Keep the reservoir floor below the inlet sill so water stays trapped when sources stop. Example: a floor 3 below the sill trapped ~650 blocks [0xguy07 map notes]. Water evaporates at `0.04608 m³ × evaporation_value` per day ([wiki Evaporation](https://timberborn.wiki.gg/wiki/Evaporation)): a ≥3-wide pool evaporates ~`0.053 m/day` per tile (adjusted adjacency score 8 → 1.16), a 2-wide channel `0.096 m/day`, and a 1-wide channel `0.200 m/day` — so avoid narrow 1-wide reservoirs and make basins deep and ≥3 tiles wide.

## Irrigation ([wiki Irrigation](https://timberborn.wiki.gg/wiki/Irrigation) & [Evaporation](https://timberborn.wiki.gg/wiki/Evaporation); confirm live with `find_planting` / `tiles moist`)
- Moisture reach from pure water (`0%` pollution) equals `2 × adjusted_adjacency_score`: up to **16 tiles** from a ≥3-wide channel or 3×3 pool. Each block of elevation increase costs **6 tiles** of reach (decreases in elevation do not reduce reach).
- Soil dries after sources stop, and crops wither, then die. Keep a pool fed by a tank and a Discharge, or a dammed reservoir, beside the fields.
- Never let badwater touch fields. At **≥ 50%** contamination, water stops irrigating and contaminates terrain.

## Design patterns
1. **First dam**: put a row of Dams across a narrow, shallow stretch downstream of the colony. It costs nothing but logs, raises the water level, widens the irrigated band, and holds a drought buffer.
2. **Levee reservoir**: ring a depression with Levees and fill it through a Floodgate. **Raise** the floodgate to its max height when the drought starts so nothing drains back (height `0` is wide open per [wiki Floodgates](https://timberborn.wiki.gg/wiki/Floodgates)). With automation, use a Weather Station or Depth Sensor.
3. **Spillway power**: water falling over a dam crest or floodgate is fast. Put water wheels in the spillway channel, never in still water.
4. **Badwater bypass**: a separate channel with its own floodgate or valve that diverts river flow around the colony during badtides (see timberborn-drought-and-badtide-response).
5. **Deep pump siting**: pumps need depth at the intake. `find_placement` with the pump prefab sorts results by `waterDepth`. Reserve those shoreline tiles. In a drought, shallow intakes run dry first.
6. **Canals and aqueducts** (mid-late): levees plus Impermeable Power Shafts plus Airlocks plus Compact Mechanical Pumps make lifted, sealed channels possible. Mechanical pumps cost a lot of power (500–700 hp), so budget for it.

## API hooks
- `GET /api/weather` for season and days left. `GET /api/tiles?x1..y2` for water depth, moisture, and contamination per tile.
- `POST /api/building/floodgate {"id","height"}` uses float heights only. `POST /api/building/storage {"id","good":"Water","mode":"obtain"}` makes a tank pull water.
- Find pump sites with `POST /api/placement/find {"prefab":"WaterPump.Folktails"|"DeepWaterPump.IronTeeth",...}`.
