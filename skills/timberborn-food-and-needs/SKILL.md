---
name: timberborn-food-and-needs
description: "Use when planning or fixing Timberborn food supply, farm sizing, kitchens, food variety, or basic needs (hunger, thirst, sleep, shelter, wet fur, injuries, contamination), or when foodDays or a need alert is low."
---

# Food and needs

## Consumption (Normal/Hard. Easy 40%)
- **2.67 food** and **2.12 water** per beaver per day. Kits count. Bots don't eat.
- Starving: adults work at half speed and kits grow 40% slower. Death comes ~3.75 days after hitting 0. Dehydrated: −25% movement, death ~4.3 days after 0.
- Hunger and thirst satisfied give +1 wellbeing each. Unsatisfied gives **−10** each. Shelter is +1/−3, sleep +1/−1 [0xguy07 from the wiki]. Food is about avoiding disaster. Wellbeing comes from **variety** and the other needs.

## Sizing farms
Crop tiles = daily food × days to mature ÷ yield per tile, then add ~30% for planting and walking time and dry spells.
| Food | Tiles per beaver (floor) |
|---|---|
| Bread (FT) / Cattail Crackers (FT) | ~1.8 |
| Corn Rations (IT) | ~2.7 |
| Eggplant Rations (IT) | ~3.1 |
| Carrots (FT) / Grilled Spadderdock (FT) | ~3.6 |
| Kohlrabi (IT) / Grilled Potatoes (FT) | ~4.0 |
| Fermented Cassava (IT) | ~5.3 |
| Sunflower Seeds (FT) | ~6.7 |
Timberbot's rule of thumb is **1 farmhouse per ~8 beavers** with full irrigated fields. Always use `POST /api/planting/find` (or the `find_planting` tool) for irrigated spots. Never guess.

## Kitchens (16 h shift, fully staffed, inputs present)
Bakery: 80 bread a day (~30 beavers), or 128 crackers a day (~48). Gristmill: 32 flour a day, enough for 2 Bakeries. Grill: ~45 beavers. Food Factory: corn ~60 beavers, eggplant ~72. Fermenter: cassava ~30, soybean ~40. Kitchens burn **0.1 log per cycle**. A busy Food Factory uses ~3.2 logs a day, so keep wood flowing [citybuildertools].

## Phases
1. **Day 0–5**: Gatherer Flags on berry bushes (finite, so they're only a bridge), then plant raw crops: Carrots (FT) or Kohlrabi (IT).
2. **First cycle**: add a second raw crop and the first processed food (Grill and potatoes, or Fermenter and cassava).
3. **Mid-game**: staple processed foods (Bread or Corn Rations) plus 3–5 distinct foods for nutrition wellbeing. Store food for drought.
4. **Late**: aquatic crops (FT), hydroponics plus algae (IT, drought-proof), tree foods (chestnut, maple, mangrove), coffee (IT, +3, not hunger).

## Other needs and how to meet them
| Need / ailment | Fix |
|---|---|
| Sleep / Shelter | enough beds. Homelessness costs wellbeing and lives |
| Wet Fur | Shower (FT) / Double Shower (IT). 1.1: Domed Garden (FT) and Dance Pit (IT) also count |
| Injury (−2) | Medical Bed. Comes from thorns, succulents, and explosions |
| Chipped teeth / bee sting (−1) | Teeth Grindstone. Keep beehives away from housing |
| Contamination (−10) | Herbalist and Antidotes (FT) or Decontamination Pod (IT). Keep beavers out of badwater |

## Storage
Food goes in warehouses (one good type per storage building). Pin warehouse goods with `POST /api/building/storage`. Keep food storage near housing and the DC to shorten haul trips. The 1.1 Goods Statistics tab shows 10-day stock charts, and `good_history` (save MCP) gives the same offline.

## Alarms
- foodDays < hazard length + 3 during temperate: add farms, staff farmhouses, raise priority.
- A crop shows "withering": irrigation failed, so fix the water before replanting.
- Starving or Dehydrated status on anyone: go to timberborn-crisis-recovery.
