---
name: timberborn-crisis-recovery
description: "Use when a Timberborn colony is in trouble: beavers dying, food or water at or near 0, mass starvation/dehydration, contamination outbreak, power collapse, construction deadlock, homelessness, or a death spiral, to triage and recover."
---

# Crisis recovery

**The death spiral**: food or water hits 0, beavers die, there are fewer workers, production drops, and more die. Starving workers run at half speed. Act at the *first* sign of trouble, not at the first death. Since 1.1 there's a blinking death alert for non-old-age deaths, and bodies stay visible for 24 h.

## Triage order (pause first: `set_speed 0`)
1. **Water** (dead in ~5.7 days from empty)
2. **Food** (dead in ~5 days from empty)
3. **Contamination** (−10 wellbeing, spreads through badwater)
4. **Shelter**
5. Everything else

## Playbooks
### A. Water emergency (waterDays ≤ 1)
1. Staff **every** pump (`set_workers` to max) and set pumps' workplace priority to VeryHigh.
2. Pause every non-essential water consumer: Hydroponics, Coffee, Refinery, Breeding Pods, Discharges, Steam Engines.
3. If intakes are dry: drop pumps on the deepest remaining water (`find_placement` pump, sort by `waterDepth`) even if it's far. Tanks with `supply` near housing.
4. Badtide: pumps won't take badwater. Find a clean pool and protect it with a levee or floodgate.
5. Free labour from construction (pause sites) so more beavers haul water.
### B. Food emergency (foodDays ≤ 1)
1. Gatherer Flags on every reachable berry patch. Harvest ripe crops (farmhouse priority VeryHigh).
2. Make kitchens turn stored raw crops into multiplied food (1 wheat → 5 bread, 1 corn → 5 rations) and keep their log fuel in stock.
3. Stop population growth: FT, don't build new beds yet. IT, pause Breeding Pods.
4. Plant the fastest crops: Kohlrabi 3 d (IT) or Carrot 4 d (FT), only on soil `find_planting` confirms is moist.
### C. Contamination outbreak
Close Gates on paths through badwater. Re-route paths. Herbalist and Antidotes (FT) or Decontamination Pod (IT). Contamination Sensor plus Fill Valve to keep clean pools clean.
### D. Power collapse
`GET /api/power`: find networks with demand > supply. Pause low-priority consumers (Clutch them off). Staff power wheels. In a drought, water wheels are dead, so switch to muscle, wind, steam, or the battery.
### E. Construction deadlock
No planks because the mill is unstaffed because no beavers are free because everyone is building or hauling. Break it: pause all construction except the Lumber Mill and its power, set the mill to VeryHigh, then resume one site at a time. Check sites are ≤ 10 tiles from a path. Check builders' huts are staffed.
### F. Homelessness / runaway population
Build cheap housing first (Lodge or Rowhouse), then migrate only up to free beds. FT: births follow free beds, so don't leave too many spare beds during a shortage.
### G. Labour misallocation
Idle adults in one district while another is short: build beds in the short district, then `migrate` up to free beds, then repeat.

## Stabilization criteria (before unpausing at speed > 1)
waterDays ≥ 3 and rising, foodDays ≥ 3 and rising, no Starving or Dehydrated beavers, alerts for unstaffed pumps and farms = 0.

## When to give up and reload
If population < ~5 adults with no water source reachable before death timers end, or `game.over` fires: recommend loading the last good save (the human must do it), and record the cause in persistent memory so the plan changes next time.

## Post-mortem template
cause → first missed signal (day, metric) → threshold to add → circuit or manager to add → storage target change.
