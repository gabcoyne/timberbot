---
name: timberborn-power
description: "Use when a Timberborn building needs power (hp) or the grid is short: choosing generators (power wheels, water wheels, wind turbines, steam/geothermal engines), routing power shafts, batteries, clutches, and sizing the network."
---

# Power (horsepower, hp)

## Rules
- Power travels only through **adjacent** power buildings and **Power Shafts** (1 log). Vertical Power Shaft: 2 log, 2 plank, 1 gear, 40 SP. **Paths do not carry power.**
- A shortage *slows* powered buildings. A disconnected building stops. Paused, unstaffed, or off-shift buildings draw nothing [0xguy07].
- `find_placement` returns `nearPower`. `GET /api/power` lists networks. WS fires `power.network.created/removed` and `power.generator.added/updated`.
- Use a Power Meter (10 gear, 6 metal block, 600 SP) to read supply, demand, surplus, and battery %. A Clutch (2 gear, 6 metal block, 400 SP) connects or disconnects part of a grid by hand or by signal (`POST /api/building/clutch`).

## Generators
| Generator | Faction | Output | Notes |
|---|---|---|---|
| Power Wheel | FT / IT | 50 hp nominal (35–85 by walker speed, so wellbeing raises it), 1 worker | 20 logs, 0 SP. Bots can't run it |
| Large Power Wheel | IT | 300 hp nominal, 4 workers (~75 each, up to ~510) | 50 logs |
| Water Wheel | FT | ~270 hp per m³/s of **current** | dead in still water |
| Compact Water Wheel | IT | ~120 hp per m³/s | |
| Large Water Wheel | IT | ~540 hp per m³/s | |
| Wind Turbine | FT | 150 peak / ~68 average, 120 SP | depends on wind (`wind.changed` event) |
| Large Wind Turbine | FT | 300 peak / ~144 average, 1400 SP | |
| Steam Engine | IT | ~400 hp, burns logs (+ water per faction post) | 20 log, 25 gear, 40 metal parts |
| Geothermal Engine | both | **400 hp** continuous, unmanned ([wiki Geothermal Engine](https://timberborn.wiki.gg/wiki/Geothermal_Engine)) | 50 log, 30 plank, 160 SP. Only on a Geothermal Field |
| Gravity Battery | both | stores 4,000 hph at ground level + 2,000 per block of clearance under the weight, 400 SP | buffers wind and drought |
(Water and wind figures from 0xguy07's measurements and wiki infobox values. Treat them as ±20%.)

## Common loads (from wiki infoboxes, via citybuildertools)
Lumber Mill 50 · Industrial Lumber Mill 75 · Gear Workshop 120 · Gristmill 60 · Paper Mill 80 · Printing Press 150 · Food Factory 120 · Fermenter 50 · Oil Press 50 · Smelter 200 · Centrifuge 200 · Wood Workshop 250 · Bot Assembler 250 · Bot Part Factory 150 · Observatory 200 · Coffee Brewery 200 · Grease Factory 200 · Dirt Excavator 200 · Explosives Factory 150 · Aquifer Drill ~400 · Mechanical Pump 500 (1.1) · Compact Mechanical Pump 700 (1.1).

## Playbook
1. **Early (FT)**: one Power Wheel next to the Lumber Mill (50 = 50). Add a wheel for each new 50 hp load, or move to Water Wheels in a dam spillway as soon as there is a steady current.
2. **Early (IT)**: one Large Power Wheel feeding a compact industrial block: 2 Gear Workshops (240) or 4 Industrial Lumber Mills (300).
3. **Cluster industry** around one shaft spine. Fewer, bigger networks are easier to read and balance.
4. **Drought-proof the grid**: water wheels stop when rivers stop. Back them up with wind and a Gravity Battery (FT), Steam Engines (IT), muscle wheels, or Geothermal. Use a Power Meter and Relay to pause low-priority consumers when the battery drops below ~30%.
5. Oasis-style maps (standing water): skip water wheels and go muscle plus wind or steam.
6. Size with ~25% headroom over the sum of loads that run at the same time.
