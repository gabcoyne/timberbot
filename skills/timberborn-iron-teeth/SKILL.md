---
name: timberborn-iron-teeth
description: "Use when the colony's faction is Iron Teeth (industrial faction, breeding pods, rowhouses/barracks, large power wheels, steam engines, metal parts, tubeways, rations) and the bot needs faction-specific buildings, prefab names, chains, and priorities."
---

# Iron Teeth playbook

Iron Teeth ("Work hard, work hard") are industrial and dense. They breed kits in **Breeding Pods**, not houses, and can even grow adults straight from the tank. They have stackable industrial storage, narrow Rowhouses, **Deep Water Pumps**, **Large Power Wheels**, **Large and Compact Water Wheels**, and **Steam Engines** that burn logs. 1.0 added **Metalsmith** and **Metal Parts**. Transport is **Tubeways**, which can run vertically and through water or badwater. Bots need a **Control Tower** signal and **Charging Stations**, and Grease boosts them. The Wonder is the **Earth Repopulator**. Iron Teeth are usually unlocked by reaching wellbeing 15 with Folktails [0xguy07, unverified for 1.1].

## Key prefabs (verify with `prefabs | grep`)
| Role | Prefab | Notes |
|---|---|---|
| Housing | `Rowhouse.IronTeeth` (1×2) · `Barrack.IronTeeth` (3×2, unlock) | capacity is **disputed**: the Timberbot guide says Rowhouse 2, a wiki-sourced search says Rowhouse 5 and Barrack 10. Use in-game `beds` |
| Breeding | Breeding Pod | consumes water + berries (wiki-sourced: 5 each, stores 2, ~5 days per kit when continuously supplied. **Unverified**). 1.1 allows line placement |
| Farming | `FarmHouse.IronTeeth` | 2 workers, 20 logs, 0 SP |
| | `HydroponicGarden.IronTeeth` | 200 SP. Mushrooms (40 water → 45 per 192 h) and algae (60 → 70 per 288 h) |
| Water | `DeepWaterPump.IronTeeth` | 3×2, 12 logs, ~48 water a day, reaches deeper water |
| Power | `LargePowerWheel.IronTeeth` 300 hp (4 workers, 50 logs) · Compact Water Wheel ~120 hp per m³/s · Large Water Wheel ~540 hp per m³/s · `SteamEngine.IronTeeth` ~400 hp, burns logs (20 log, 25 gear, 40 metal parts) | |
| Wood | `LumberjackFlag.IronTeeth`, `Forester.IronTeeth`, `IndustrialLumberMill.IronTeeth` (2 workers, 75 hp, 1 plank per 0.63 h ≈ 25 a day) | |
| Storage | `SmallIndustrialPile.IronTeeth` (not SmallPile), `MediumWarehouse.IronTeeth` (not SmallWarehouse), `SmallTank.IronTeeth` | |
| Metal | `ScavengerFlag.IronTeeth` (**free from the start** since 1.0) → `Metalsmith` (150 SP, 1 scrap + 0.2 log → 1 metal part per 3 h) · `Smelter` (300 SP, 2 scrap + 0.2 log → 1 metal block per 2 h, 200 hp) | |
| Science | `Inventor.IronTeeth`, Numbercruncher (30 plank, 100 gear, 50 metal parts) | |
| Health | Decontamination Pod (cures contamination), TeethGrindstone, MedicalBed | |

## Food chains [citybuildertools]
| Food | Crops | Kitchen | Note |
|---|---|---|---|
| Kohlrabi (raw) | 3 d, 2 per tile | none | day-1 crop |
| Fermented Cassava | Cassava 5 d, 1 | Fermenter (0 SP, 50 hp): 4 → 10 per 2 h | early processed food |
| Fermented Soybeans | Soybean 8 d + Canola Oil | Fermenter: 6 soy + 1 oil → 20 per 3 h | |
| Corn Rations | Corn 10 d, 2 | Food Factory (300 SP, 120 hp): 1 corn + 0.1 log → 5 per 0.5 h | best per tile, ~60 beavers per factory |
| Eggplant Rations | Eggplant 12 d + Canola Oil | Food Factory: → 6 | ~72 beavers per factory |
| Algae Rations | Hydroponic algae + oil | Food Factory | drought-proof indoor food |
| Mangrove Fruit | mangrove tree, 4 every 10 d | none | |
| Coffee | Coffee bush → Coffee Brewery (500 SP): 1 bean + 1 water → 4 | | +3 nutrition, does **not** satisfy hunger |
Canola comes from an Oil Press (120 SP, 40 log, 10 metal parts, 1 seed → 1 oil per 1.3 h).

## Wellbeing ceiling (pre-1.1 table: ~77) [Timberbot guide]
BasicNeeds 5 (WetFur via DoubleShower) · Social 2 (Campfire, Rooftop Terrace) · Fun 17 (Scratcher 1, Swimming Pool 1, Exercise Plaza 3, Mud Bath 3, Wind Tunnel 3, Motivatorium 5) · Nutrition 17 · Aesthetics 10 · Awe 26 (Laborer Monument 3, Flame of Unity 5, Tribute to Ingenuity 8, Earth Repopulator 10).
**1.1 additions**: Massager (150 SP, 10 plank, 5 gear), Dance Pit (1800 SP, 60 plank, 20 gear, 30 metal parts, also Wet Fur), Tier 4 **Arch of Progress** (15,000 SP, 600 plank, 300 treated plank, 600 metal block), and the **Impermeable Tubeway** (800 SP) that fits inside levees. Wind Tunnel now costs 100 plank, 50 gear, 50 metal block, 800 SP. Exact wellbeing points and the new max are unverified.

## Iron Teeth-specific priorities
1. Breeding is something you build, not something that happens. Keep Breeding Pods supplied, and pause them when beds or food run short. Pods are your population throttle.
2. Get power early. One Large Power Wheel (300 hp) runs 4 Industrial Lumber Mills, or 2 Gear Workshops with 60 hp left over.
3. Run the scavenger and Metalsmith from the start. Many mid-tier Iron Teeth buildings cost Metal Parts since 1.0 (Steam Engine, Oil Press, Hydroponic Garden, Charging Station, Exercise Plaza, Numbercruncher, Flame of Unity).
4. Longer shifts suit Iron Teeth, but check that pumps and farms keep up, since consumption is per day, not per shift.
5. Hydroponics plus Algae Rations give food that ignores droughts and badtides. That is the core of late-game Hard resilience.
