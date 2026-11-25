---
name: timberborn-fundamentals
description: "Use first whenever a bot is about to play, plan, or advise on Timberborn (v1.1) and needs the core rules of the game, its vocabulary, survival math, and a map to the other skills."
---

# Timberborn fundamentals (game version 1.1.x)

Timberborn (Mechanistry) is a city builder about beavers in a post-human wasteland. Version 1.0 left Early Access on 2026-03-12. Update 1.1 reached the main branch on 2026-09-03 (build 1.1.2.4), and small main-branch patches followed (2026-10-06 and 2026-10-20). Everything here targets 1.1. If the live game disagrees with a number, trust the game and update the skill.

## The core loop
1. **Water** keeps beavers alive. Rivers flow during the *temperate* season and stop or turn poisonous during *hazardous* seasons (droughts and badtides).
2. **Food** comes from berries early, then from irrigated crops. Crops need soil kept moist by nearby water.
3. **Logs, then planks, then gears** pay for nearly every building. Treated planks, metal blocks, and extract come later.
4. **Science points (SP)** come from Inventors (1 SP per hour per worker) and later the Observatory. Spend SP to unlock buildings.
5. **Wellbeing** comes from needs met above survival. It multiplies work speed (up to +260%), movement, and lifespan. This is the main way to scale.
6. **Endgame**: Tier 1 to 4 monuments, then a Wonder. Building and *launching* a Wonder completes the map.

## Survival numbers to memorize (Normal/Hard. Easy uses 40% of consumption)
| Quantity | Value | Source |
|---|---|---|
| Food per beaver per day | ~2.67 items (hunger drops 80 points a day, each item restores 30) | citybuildertools, from the wiki |
| Water per beaver per day | ~2.12 (thirst drops 70 a day, each drink restores 33) | same |
| Time to death after food runs out | starving after ~1.25 days, dead ~3.75 days later (~5 days total) | same |
| Time to death after water runs out | dehydrated after ~1.4 days, dead ~4.3 days later (~5.7 days total) | same |
| Water Pump / Deep Water Pump output | 1 water every 0.33 h per worker, so ~48 a day on a 16 h shift (~22 beavers) | same |
| Small / Medium / Large Tank | 30 / 300 / 1200 water | same |
| Temperate length (Easy/Normal/Hard) | 16–19 / 13–17 / 5–8 days | wiki Game Mode, via search. Verify in game |
| Drought length (Easy/Normal/Hard) | 2–4 / 5–9 / 15–30 days, ramping up over the first ~6 cycles (handicap) | same |
| Badtides | Easy: 1–3 d (40% after 5 cycles). Normal: 4–8 d (40% after 4 cycles). Hard: 15–30 d (40% after 3 cycles) | [wiki Game Mode](https://timberborn.wiki.gg/wiki/Game_Mode) |
| Kits, adults, bots | kits eat and drink. Bots eat and drink nothing | wiki |

Kits and adults both consume. Working hours do **not** change consumption, but shorter shifts cut pump and farm output.

## Vocabulary the bot must keep straight
- **Cycle** = one temperate season plus one hazardous season. A game always starts temperate.
- **Drought** = sources stop. **Badtide** = sources emit *badwater* (contaminated). It kills crops, contaminates beavers that wade through it, and pollutes soil above ~50% contamination.
- **District** = a District Center plus everything connected to it by paths. Goods do not cross districts unless you use District Crossings and distribution settings.
- **Haulers** are idle beavers plus Hauling Post workers. They move goods. If every beaver has a job, nobody hauls.
- **Builders** are District Center workers plus Builders' Huts. They build only within ~10 tiles of a connected path [0xguy07 mechanics notes].
- **Priorities** (VeryLow…VeryHigh) control staffing and build order. **Automation signals** are On/Off. **Floodgate heights** are floats. Never mix them up (see timberborn-automation-circuits).
- **Prefab names** carry a faction suffix, such as `LumberjackFlag.Folktails`. `Path` and the `Reserve*` storages are the exceptions. Look names up instead of guessing (see timberborn-control-timberbot-api).

## Coordinates and placement basics
- Grid X/Y is horizontal and Z is height. A building's `z` must equal the terrain height at its origin.
- Every building has an **entrance tile** outside its footprint, and that tile must be a path connected to the District Center. Example: an unrotated Lodge at origin (x,y,z) has its doorstep at (x+1, y−1, z) [0xguy07].
- Buildings flood and stop working if water touches their footprint. Paths, power shafts, and landscaping are immune.
- Beavers cannot climb. Height changes need Stairs, Spiral Stairs, Platforms, Ziplines (Folktails), or Tubeways (Iron Teeth).

## Iron rules
1. Never let water or food reach 0. The death spiral is fast: fewer beavers, less output, more deaths.
2. Reserve waterfront tiles for pumps. Do not run paths or put other buildings along the shore.
3. Pause (speed 0) to plan, give everyone work, then unpause. An idle colony burns food and water for nothing.
4. Run mutating actions one at a time and check each result before the next.
5. Size tanks before the drought, not during it.

## Skill map
- Control: timberborn-agent-loop, timberborn-control-timberbot-api, timberborn-control-http-levers, timberborn-save-file-analysis
- Factions: timberborn-folktails, timberborn-iron-teeth
- Systems: timberborn-water-management, timberborn-food-and-needs, timberborn-power, timberborn-logistics-and-districts, timberborn-science-and-unlocks, timberborn-automation-circuits, timberborn-monuments-and-wellbeing
- Phases: timberborn-map-reading-and-site-selection, timberborn-build-order-early-game, timberborn-mid-late-game-expansion
- Emergencies: timberborn-drought-and-badtide-response, timberborn-crisis-recovery
- Data: timberborn-production-reference
