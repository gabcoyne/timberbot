---
name: timberborn-mid-late-game-expansion
description: "Use once a Timberborn colony has survived its first hazards with stable food/water/planks (roughly 20+ beavers) and needs a plan for scaling: industry tiers, metal, treated planks, bots, districts, vertical building, badtide readiness, monuments and the wonder."
---

# Mid and late game expansion

## Mid game (cycles ~3–8, 20–80 beavers)
Goals: hazard-proof the colony, open the material ladder, and push wellbeing past 15–20.
1. **Water at scale**: reservoir behind levees and floodgates, Medium then Large tanks, storage sized to the *next* hazard × 1.25. Add a badwater bypass before badtides can start (Normal: after ~4 cycles).
2. **Material ladder** [citybuildertools]:
   - Planks: 1 FT mill ≈ 12 a day, 1 IT mill ≈ 25 a day.
   - Gears: 1 workshop ≈ 5.3 a day (120 hp). For 10 a day you need 2 workshops plus 1 FT mill (or 1 IT mill). Gears are usually what the queue waits on.
   - Treated planks: Tapper's Shack (500 SP, ~20–22 resin a day with ~75 mature pines in reach), then Wood Workshop (800 SP, 250 hp, ~5.3 a day each).
   - Metal: Scavenger on ruins (`RuinColumnH<N>` gives N×15 scrap), then Smelter (8 metal blocks a day, 200 hp). IT also needs Metalsmith metal parts.
   - Extract: Badwater Pump, then Centrifuge (600 SP, ~21 a day).
3. **Food variety**: 3–5 distinct foods. FT: carrots, bread, grilled potatoes, sunflower, crackers. IT: kohlrabi, fermented cassava or soy, corn rations, eggplant rations.
4. **Wellbeing**: Campfire, Shower, aesthetics (Lanterns, Shrubs, Roofs) near housing and paths. Then Fun (Carousel FT / Exercise Plaza IT). Then Knowledge (FT books). See timberborn-monuments-and-wellbeing.
5. **Labour**: Builders' Huts at High during build waves. Hauling Posts near industry. Keep 10–20% slack.
6. **Automation**: Weather Station plus floodgates, Resource Counter water-low alarm, and pausing lumber mills when logs run low (see timberborn-automation-circuits).

## Late game (cycles 8+, 80–600+ beavers)
1. **Second and third districts** on separate resource regions, each with its own water, food, housing, and wood. District Crossings plus distribution rules. Migrate only up to free beds.
2. **Vertical density**: Spiral Stairs columns (1 per ~8–10 floor tiles per level). Housing on top, workshops in the middle near power, gathering at ground level. Any building with a stackable top is a foundation, and **deleting a support destroys everything above it** [0xguy07]. Build wide before tall.
3. **Bots**: Bot Part Factory plus Assembler. FT bots need biofuel (Refinery, potato recipe). IT bots need a Control Tower and Charging Stations. Bots work 24/7 with no needs. Use them for hauling, industry, and dangerous jobs (thorns, explosives). Not for Power Wheels or Inventors.
4. **Terraforming**: Dirt Excavator (2000 SP) for dirt. Explosives Factory plus Detonator for blasting terrain. Watch for Unstable Cores (blast radius ≤ 5).
5. **Mines** once ruins run out: Mine (FT) or Efficient Mine (IT), 4000 SP, 10 workers, 1 treated plank → 5 scrap per 1.8 h (IT efficient recipe: +2 extract → 10 scrap).
6. **Advanced water**: Compact Mechanical Pumps, Airlocks, Impermeable shafts and tubeways, and aqueducts to green the map.
7. **Monuments Tier 1–4**, then the **Wonder** (build, then launch, to complete the map and unlock Flexible Start).

## Scaling ratios (approximate. Check against `workplace_report`)
| Item | Ratio |
|---|---|
| Basic pump | 1 per ~20 beavers for drinking + extra to fill tanks |
| Farmhouse | 1 per ~8 beavers (full irrigated fields) |
| Lumberjack | 1 per ~15 beavers. Forester 1 per 4 lumberjacks |
| Gear Workshop | 1 per ~5 gears a day of demand |
| Hauling Post | 1 per busy industrial cluster |
| Inventors | 3–6 until Observatory / Numbercruncher |

## Expansion gate (all must be true before a big expansion)
waterDays ≥ next hazard × 1.25 · foodDays ≥ next hazard + 3 · plankDays ≥ 3 · no homeless · alerts clear · wellbeing not falling.
