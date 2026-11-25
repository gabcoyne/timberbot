---
name: timberborn-science-and-unlocks
description: "Use when deciding what to research next in Timberborn, how to raise science-point (SP) income, or when a placement fails with not_unlocked / insufficient_science."
---

# Science and unlock order

## Income
- **Inventor** (12 logs, 0 SP, 1 worker): 1 SP per hour of work, so ~16 SP a day on a 16 h shift. Beavers only (bots can't work it). Build 1 by day ~3, then 2–4 in the first cycles. Cheap and always worth it until the Observatory.
- **Observatory** (FT, 1000 SP, 80 plank, 30 gear, 10 pine resin, 4 workers, 200 hp): 10 SP per cycle, so **12 SP an hour** since 1.1.
- **Numbercruncher** (IT, 30 plank, 100 gear, 50 metal parts): Iron Teeth's late science building. Power demand is **400 hp** ([wiki Power](https://timberborn.wiki.gg/wiki/Power)); exact unlock SP and science/hr output remain unverified.
- **Relics** (small, medium, large) on some maps grant free SP when collected.
- Wellbeing raises work speed, and science scales with it.
- HTTP Lever and Adapter unlocks (5000 and 7500 SP) are **global**: you pay once for all future settlements.

## Unlock costs (SP) worth knowing (1.0/1.1 wiki and patch notes)
| Tier | Unlocks |
|---|---|
| 0 | Water Pump, Deep Water Pump, Lumber Mill, Industrial Lumber Mill, Efficient Farmhouse, Farmhouse, Grill, Fermenter, Inventor, Power Wheel, Large Power Wheel, Dam, Hauling Post, Scavenger Flag (IT) |
| ≤ 200 | Forester 30 · Vertical Power Shaft 40 · Lever 50 · Relay 80 · Gear Workshop 100 · Memory 100 · Flow Sensor 100 · Levee 120 · Oil Press 120 (IT) · Wind Turbine 120 (FT) · Floodgate 150 · Aquatic Farmhouse 150 (FT) · Metalsmith 150 (IT) · Chronometer 150 · Massager 150 (IT, 1.1) · Geothermal Engine 160 · Bakery 160 · Gristmill 180 · Gate 200 · Depth Sensor 200 · Population Counter 200 · Hydroponic Garden 200 (IT) |
| 250–600 | Resource Counter 250 · Badwater Pump 250 · Paper Mill 250 · Discharge 250 · Double Floodgate 250 · Scavenger Flag 250 (FT) · Smelter 300 · Fill Valve 300 · Food Factory 300 · Weather Station 300 · Science Counter 300 · Herbalist 300 · Airlock 300 · Impermeable Power Shaft 300 · Sauna 300 (FT) · Spiral Stairs 350 · Builders' Hut 400 · Large Water Pump 400 · Refinery 400 · Printing Press 400 · Explosives Factory 400 · Aquifer Drill 400 · Gravity Battery 400 · Contamination Sensor 400 · Clutch 400 · Indicator 400 · Triple Floodgate 500 · Throttling Valve 500 · Tapper's Shack 500 · Coffee Brewery 500 · Bot Part Factory 500 · Speaker 500 · Centrifuge 600 · District Crossing 600 · Power Meter 600 · Timer 600 · Carousel 600 |
| 700–2000 | Firework Launcher 700 · Banners 700 · Bot Assembler 750 · Wood Workshop 800 · Wind Tunnel 800 · Impermeable Tubeway 800 (IT) · Farmer / Laborer Monument 1000 · Observatory 1000 · Large Wind Turbine 1400 · Detonator 1400 · Dance Pit 1800 (IT) · Mechanical Pump 2000 · Badwater Dome 2000 (FT) · Grease Factory 2000 (IT) · Dirt Excavator 2000 |
| 3000+ | Brazier of Bonding / Flame of Unity 3000 · Mine / Efficient Mine 4000 · Badwater Rig 4000 · Compact Mechanical Pump 4000 · HTTP Lever 5000 · HTTP Adapter 7500 · Fountain of Joy / Tribute to Ingenuity 12000 · Hall of Abundance / Arch of Progress 15000 · Earth Recultivator / Repopulator 20000 |
Read live costs with `GET /api/science` and unlock with `POST /api/science/unlock {"building":"<prefab>"}`. Errors: `insufficient_science` (+scienceCost, currentPoints) or `not_found`.

## Recommended order (adapt to the map)
**Both factions, cycle 1–2:** Forester (30), then Gear Workshop (100), then Levee (120) and Floodgate (150), then Inventors x2–3 (free). Add Stairs or Spiral Stairs early if the start is terraced.
**Folktails:** Bakery and Gristmill (wheat staple) or Aquatic Farmhouse (cattail), then Wind Turbine (drought power), then Medium/Large tanks, then Builders' Hut, then Scavenger and Smelter, then Tapper's Shack and Wood Workshop (treated planks), then Observatory, then Herbalist before badtides begin (Normal: after cycle ~4), then Large Water Pump, then District Crossing, then monuments.
**Iron Teeth:** Metalsmith (150) right away (metal parts gate many IT buildings), then Oil Press and Food Factory (rations), then Smelter, then Hydroponic Garden, then Steam Engine, then Builders' Hut, then Tapper's Shack and Wood Workshop, then Decontamination before badtides, then District Crossing, then monuments, then Efficient Mine once ruins run out.
**Automation starter pack** (cheap and high value): Lever 50, Relay 80, Depth Sensor 200, Weather Station 300, Resource Counter 250.

## Decision rule
Unlock whatever removes the **current binding constraint** (water storage, food variety, planks, gears, power, metal, or labour), not whatever is cheapest. Keep a reserve equal to the next survival-critical unlock before the hazard season.
