---
name: timberborn-drought-and-badtide-response
description: "Use when a drought or badtide is approaching, active, or just ended in Timberborn, or when a weather event fires (drought.approaching, drought.start, drought.end), to run the prepare / endure / recover checklists."
---

# Drought and badtide response

Every cycle is one temperate season followed by one hazardous season, either a **drought** or a **badtide**. Hazard length grows over the first cycles through the difficulty "handicap", which starts at a percentage of max severity and climbs to 100% (Normal default: drought handicap 38% over 6 cycles) [wiki Game Mode via search, verify]. The best forecast is the game's own recent history, which you can get from saves (`overview`) or `GET /api/weather`.

| Difficulty | Temperate | Drought | Badtide |
|---|---|---|---|
| Easy | 16–19 d | 2–4 d | (unverified) |
| Normal | 13–17 d | 5–9 d | 40% chance per cycle after 4 badtide-free cycles |
| Hard | 5–8 d | 15–30 d | 40% after 3 cycles |
(All wiki-sourced via search. Confirm on the New Game screen or the wiki.)

## Detecting it
- WS events: `weather.selected` (the next hazard type is chosen), `drought.approaching` (the UI warning), `drought.start` and `drought.end` (fired for both drought and badtide), `cycle.start`.
- `GET /api/weather` and `GET /api/summary?format=json` fields: `isHazardous`, `temperateWeatherDuration`, `hazardousWeatherDuration`, `cycleLengthInDays`, `cycle`, `cycleDay` (compact `?format=toon` summary uses `tempDays` / `hazardDays`). Note: `isHazardous` does not distinguish drought from badtide; check river tile `contamination` via `GET /api/tiles`.
- In-game: a **Weather Station** (10 gear, 8 treated plank, 300 SP) outputs On for chosen seasons. It is the best automation trigger.

## T-minus checklist (start ≥ 3 days before, ideally the whole temperate season)
1. waterDays ≥ hazard max + 2. If short: add pumps (cheapest), staff them all, set tanks to `obtain`, and raise pump priority.
2. foodDays ≥ hazard max + 3. Harvest, then stop planting crops that won't mature before soil dries unless the fields are reservoir-irrigated.
3. Finish or pause log-hungry construction so logs and planks stay ≥ 3 days.
4. **Close reservoir outlets.** Set floodgates to hold height and make sure nothing drains back into the dry river.
5. Badtide only: get the **badwater bypass** ready. Close intake floodgates or valves to clean pools and fields, open the bypass channel, close Gates on paths that ford rivers, and move pumps off intakes that will turn bad (or pause them).
6. Housing: no homeless (shelter unmet costs −3 wellbeing and slows recovery).
7. Checkpoint: ask the human to save (see timberborn-agent-loop).

## During a drought
- Water is spent, not made, except by Aquifer Drills, tanks with Discharges, and Succulents (1.1: grow on dry land, store **2 water** each since a later patch, and injure lumberjacks. A weak last resort).
- Pumps keep pulling from reservoirs until intakes run dry. Watch reservoir depth with Depth Sensors.
- Keep irrigation for the best fields: a Discharge fed from a tank can water one patch.
- Don't start big builds. Spend labour on hauling, science, and indoor production.
- If waterDays < remaining hazard days: (a) trim nonessential water users (Hydroponics, Coffee, Refinery, Breeding Pods), (b) look into short-term pumping from distant pools, and (c) as an absolute last step, shorten the workday so beavers walk less, which does *not* cut drinking. Consumption is per day.

## During a badtide
- Every source emits badwater. Contaminated water over ~50% contaminates terrain and kills plants. Beavers that wade through it get **Contaminated** (−10 wellbeing ailment).
- Keep badwater out of drinking pools (pumps only draw clean water, but a mixed pool ends up contaminated) and away from fields and forests. Barriers must be continuous, **including diagonals**.
- Cure: Folktails Herbalist (Antidotes from dandelions, berries, and paper). Iron Teeth Decontamination Pod.
- Badwater is also a resource: pump it to tanks (Badwater Pump, 250 SP) for Extract (Centrifuge) and Explosives.
- Automation recipe: Contamination Sensor (threshold ~0.5) → Relay NOT → intake floodgate or Fill Valve. Weather Station (Badtide) → bypass floodgate. Keep the diversion active until the contamination sensor reads clean, not just until `drought.end`, because tails of badwater linger.

## Recovery (first 2 days after `drought.end`)
1. Replant dead or withered crops right away (`crops`, `planting/mark`). Forester replant.
2. Reopen floodgates to refill reservoirs. During a badtide recovery, wait for the contamination sensor to read clean first.
3. Refill tanks before expanding. Use extra pumps temporarily if tanks are below 50%.
4. Review: did water or food go below 2 days? If so, raise storage targets by the shortfall plus 25% for next cycle, because the next hazard will be longer.
5. The temperate season is the time to grow: housing, new production, and science.

## Map-specific notes
- **Nomads (1.1, 256×256)**: water flow across the map changes with the cycles. Players report the best approach is a compact core near the two most stable sources, pumping and tanking heavily before each change, moving only pumps and water infrastructure, and using Discharges to irrigate [r/Timberborn reports].
- **Pressure**: sources run underground, and plugging one outlet affects others. There are multiple Unstable Cores (explosion radius up to 5 tiles).
- **Oasis**: start in an isolated green refuge. Expand by tapping Aquifers with drills. Standing water means water wheels are poor, so rely on muscle or wind.
- **Pillars**: build on the upper level and keep seep reservoirs above 0.8 during badtides.
