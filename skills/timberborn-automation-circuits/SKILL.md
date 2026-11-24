---
name: timberborn-automation-circuits
description: "Use when building or wiring Timberborn's in-game automation (1.0+): levers, relays, memory, timers, chronometer, weather station, depth/flow/contamination sensors, resource/population/science counters, power meter, gates, clutches, valves, floodgates, HTTP lever/adapter, and wiring them via the Timberbot API."
---

# Automation circuits

## Semantics (memorize)
- Wires carry **On/Off only**. A sensor compares an analog reading with a `threshold` using a `mode` and outputs On or Off.
- A pausable building with an input runs on **On**, pauses on **Off**, and runs normally when **disconnected**. A paused building frees its workers.
- **Floodgates are the exception**: the signal switches between two preset heights (On height and Off height). Throttling Valves switch between two flow rates. Badwater Domes open and close.
- Don't confuse automation with **priorities** (VeryLow…VeryHigh, staffing only) or **heights** (floats). [timberbot design/automation-states.md]
- "Tick once" key (default `\`) steps one tick to debug a circuit.

## Parts (cost; SP) [1.0 patch notes]
| Part | Cost | SP | Role |
|---|---|---|---|
| Lever | 1 plank, 2 gear | 50 | manual signal. Can be pinned. `springReturn`, `pinned` |
| Relay | 1 plank, 1 gear | 80 | Not / And / Or / Xor / Passthrough. **1.1: more than two inputs** (the API exposes A–H) |
| Memory | 1 metal block, 1 extract | 100 | SetReset / Toggle / Latch / FlipFlop. Inputs A, B, reset |
| Timer | 1 treated plank, 1 metal block | 600 | delay or oscillate a signal. Can reset on a condition (mode names unverified) |
| Chronometer | 8 plank, 6 gear | 150 | TimeRange / WorkingHours / NonWorkingHours (`startTime`, `endTime`) |
| Weather Station | 10 gear, 8 treated plank | 300 | On during the chosen season(s) (temperate, drought, badtide) |
| Flow Sensor | 6 plank, 4 gear | 100 | water speed below it (1 tile high since 1.0) |
| Depth Sensor | 4 gear, 8 scrap | 200 | water depth below it |
| Contamination Sensor | 8 gear, 8 metal block | 400 | badwater fraction below it |
| Resource Counter | 4 plank, 4 gear + 1 paper (FT) / 1 metal part (IT) | 250 | district stock or fill rate of one good |
| Population Counter | 2 plank, 2 gear + paper / metal part | 200 | beavers, bots, unemployed, contaminated, … district or global |
| Science Counter | 6 plank, 4 scrap + 6 paper / metal parts | 300 | SP threshold |
| Power Meter | 10 gear, 6 metal block | 600 | supply, demand, surplus, battery % |
| Indicator | 2 scrap, 1 pine resin | 400 | light, UI pin, journal entry, warning |
| Speaker | 4 plank, 2 gear, 4 metal block | 500 | sound alarm |
| Firework Launcher | 1 gear, 2 treated plank | 700 | celebration |
| Detonator | 1 metal block, 1 explosives, 1 extract | 1400 | fires dynamite on signal |
| Gate | 6 plank, 4 gear, 4 scrap | 200 | open or close beaver passage |
| Clutch | 2 gear, 6 metal block | 400 | connect or disconnect a power sub-grid |
| Fill Valve / Throttling Valve | see water skill | 300 / 500 | water level / flow control |
| HTTP Lever / HTTP Adapter | 2 gear, 2 scrap / 2 gear, 4 scrap | 5000 / 7500 (global) | external control (see timberborn-control-http-levers) |

## Wiring through the Timberbot fork
```
POST /api/automation/link      {"sourceId":S,"targetId":T,"input":"a".."h"|"reset"}
POST /api/automation/unlink    {"id":T,"input":"a"}
POST /api/automation/configure {"id":S,"property":P,"value":"V"}
```
| Component | Properties |
|---|---|
| DepthSensor / ContaminationSensor / FlowSensor | `threshold` float, `mode` Equal / NotEqual / Greater / GreaterOrEqual / Less / LessOrEqual |
| ResourceCounter | `goodId`, `threshold` int, `fillRateThreshold`, `mode` StockLevel / FillRate, `comparisonMode`, `includeInputs` |
| PopulationCounter | `threshold`, `mode`, `comparisonMode`, `globalMode`, `countBeavers`, `countBots` |
| PowerMeter | `mode` Power / Percent, `comparisonMode`, `intThreshold`, `percentThreshold` |
| Relay | `mode` Not / And / Or / Xor / Passthrough |
| Memory | `mode` SetReset / Toggle / Latch / FlipFlop |
| Chronometer | `startTime`, `endTime`, `mode` TimeRange / WorkingHours / NonWorkingHours |
| Lever | `springReturn`, `pinned`, `state` |
Building JSON includes an `automation` block (`state`, `config`, `outputs`, or `isAutomated`/`inputState`/`input`). Known limits: the fork cannot list full wiring graphs, and some thresholds without public setters can't be set. **Verified from `TimberbotWrite.cs`: `WeatherStation`, `Timer`, and Floodgate On/Off preset heights are not handled by `/api/automation/configure`, so set them by hand in game.** Memory in SetReset or Toggle mode rejects input B unlink (`invalid_param`).

## Proven circuits
1. **Pump guard**: Depth Sensor at the pump intake (`threshold` ≈ 0.3, `GreaterOrEqual`) → pump. The pump pauses before it runs dry and frees its worker.
2. **Smart pump**: Relay And (A = Depth Sensor depth OK, B = Resource Counter Water `Less` 80% fill) → pumps. Pumps run only when there's water to pull *and* tanks need it.
3. **Hazard floodgate**: Weather Station (Drought + Badtide) → reservoir outlet floodgate. On height = closed/hold, Off height = normal spill.
4. **Badwater rejection**: Contamination Sensor (`Greater` 0.05–0.5) → Relay Not → intake Fill Valve or floodgate to the clean pool. Add an Or with the Weather Station (Badtide) so it closes early, and keep the sensor input so it stays closed until the water is clean.
5. **Log saver**: Resource Counter Log `Less` N → Relay Not → Lumber Mills, Gear Workshops, and kitchens' optional producers. Keeps logs for construction and fuel.
6. **Overflow stop**: Resource Counter good X FillRate `GreaterOrEqual` 0.95 → Relay Not → producer of X. Frees workers for hauling.
7. **Night-only attractions or day-only industry**: Chronometer WorkingHours / NonWorkingHours → Gates or workshops.
8. **Battery guard**: Power Meter Percent `Less` 30 → Relay Not → Clutch on the low-priority sub-grid.
9. **Population brake (IT)**: Population Counter (homeless or unemployed `Greater` 0) → Relay Not → Breeding Pods.
10. **Alarm**: any critical condition → Indicator (warning + journal) and/or HTTP Adapter → bot webhook.

## Design rules
- Keep circuits local and labelled (`rename_automation` / in-game names). One concern per relay chain.
- Add hysteresis with Memory (SetReset: set at low, reset at high) so pumps and mills don't flap around a threshold.
- Test with the game paused plus "Tick once", or at speed 1, before trusting a circuit during a hazard.
- In-game circuits beat bot reflexes for anything time-critical. They run when the bot is offline.
