# Mock Fidelity & Timberborn 1.1 Source Matrix

This document records the exact provenance of every HTTP/WebSocket API shape, failure mode, and Timberborn 1.1 game mechanic modeled in `rust/crates/timberbot-mock`, `rust/crates/timberbot-protocol`, `rust/crates/timberbot-save`, and `skills/`.

No number is marked **verified** without a cited primary source. Everything else is explicitly labeled **approximated** (intentional simplification for deterministic offline CI) or **guess** / **unverified** (requires a live Timberborn 1.1 install and playtest).

## Status Legend

| Status | Meaning |
|---|---|
| **verified** | Confirmed directly from this repository's C# mod source (`timberbot/src/*.cs`), recorded golden OpenAPI fixtures (`python/tests/fixtures/openapi/*.json`), `openapi.yaml`, official Timberborn Wiki (`timberborn.wiki.gg`), Mechanistry Update 1.1 patch notes, or `0xguy07/timberborn-tools` (verified on 1.1.2.4). |
| **approximated** | Real-game value or rule is known, but `timberbot-mock` intentionally uses a simplified deterministic model (e.g., 1D river depth instead of 3D Unity fluid solver, simplified path reachability) so offline tests run fast and repeatably. |
| **guess** | Not yet proven from primary public docs or a live Timberborn 1.1 binary; treated as unverified and flagged for real-game confirmation. |

---

## 1. Mod HTTP, WebSocket & Failure-Mode Fidelity

| Surface / Behavior | Real-Game Source | Status | Mock Notes |
|---|---|---|---|
| HTTP port `8085`, WS port `8086` (`/api/ws`), **no `POST /mcp` on mod** | `timberbot/src/TimberbotService.cs:63-65`, `TimberbotHttpServer.cs`, `openapi.yaml` | **verified** | Mock binds separate HTTP + WS listeners; MCP runs out-of-process in `timberbot-mcp` (stdio) or `tbot serve` (`127.0.0.1:8091/sse`). |
| Non-loopback `listenAddress` refuses to start when `authToken` is empty | `timberbot/src/TimberbotPure.cs` (`RequiresAuthToken`), `TimberbotService.cs` | **verified** | `timberbot_protocol::requires_auth_token` + `RunningMockServer::start` mirror the exact guard. |
| Bearer auth: `401` + `WWW-Authenticate: Bearer realm="timberbot"` + `{"error":"unauthorized: missing or invalid bearer token"}`; `/api/ping` exempt | `timberbot/src/TimberbotHttpServer.cs:259-273` | **verified** | Constant-time token comparison via `subtle::ConstantTimeEq`. |
| Ready gate: `409` + `{"error":"game_not_ready","hint":"player must press Launch in the Timberbot widget"}`; exempt: `/api/ping`, `/api/ready`, `/api/agent/*` | `timberbot/src/TimberbotHttpServer.cs:279-285`, `TimberbotAgentState.cs:356-371` | **verified** | Exact byte-compatible 409 JSON body. |
| Body size cap (`maxBodyBytes`, default `1048576`): `413` + `{"error":"body_too_large: ..."}` | `timberbot/src/TimberbotHttpServer.cs:365`, `TimberbotService.cs:74` | **verified** | Enforced in `timberbot-mock` HTTP server. |
| POST write-job queue: background HTTP listener enqueues to `ConcurrentQueue`; Unity main thread drains up to 10 POSTs/frame (`_writeBudgetMs = 1.0`) | `timberbot/src/TimberbotHttpServer.cs:6,139-225`, `TimberbotService.cs:67` | **approximated** | Mock serializes writes through an async FIFO write queue with configurable drain/latency so queue ordering and backpressure are testable without Unity's frame loop. |
| `GET /api/ping` → `{"status":"ok","ready":true,"openapiVersion":"1.0.0"}` | `timberbot/src/TimberbotHttpServer.cs:287-290`, `python/tests/fixtures/openapi/ping.json` | **verified** | Byte-compatible JSON shape; `ready: true` indicates HTTP server readiness (distinct from `agent/state.ready`). |
| `GET /api/settlement`, `/api/time`, `/api/weather`, `/api/speed`, `/api/workhours`, `/api/agent/state` | `timberbot/src/TimberbotReadV2.cs`, `python/tests/fixtures/openapi/{settlement,time,weather,speed,workhours,agent_state}.json` | **verified** | Exact JSON keys verified against recorded golden fixtures. Note `/api/weather` JSON uses `temperateWeatherDuration`/`hazardousWeatherDuration`/`cycleLengthInDays` (whereas compact `?format=toon` summary uses `tempDays`/`hazardDays`). |
| `GET /api/summary?format=json` (`alerts`, `buildings`, `crops`, `districts`, `faction`, `foodClusters`, `science`, `settlement`, `time`, `treeClusters`, `trees`, `weather`, `wellbeing`) | `timberbot/src/TimberbotReadV2.cs`, `python/tests/fixtures/openapi/summary.json` | **verified** | Mock matches all recorded top-level keys including `trees` and `wellbeing` as well as district `population`, `resources`, `housing`, `employment`, `wellbeing`, and `dc`. |
| `GET /api/buildings?format=json` (`items`, `limit`, `offset`, `total`, including `effectRadius`, `isWonder`, `needsNutrients`, `allowedGood`, `currentRecipe`, `automation`) | `timberbot/src/TimberbotReadV2.cs`, `python/tests/fixtures/openapi/buildings.json` | **verified** | Exact field names and types aligned with golden `buildings.json`. |
| `GET /api/beavers?format=json` (`items`, `limit`, `offset`, `total`, including `carryAmount`, `carrying`, `liftingCapacity`, `needs`) | `timberbot/src/TimberbotReadV2.cs`, `python/tests/fixtures/openapi/beavers.json` | **verified** | Exact field names and types aligned with golden `beavers.json`. |
| `GET /api/tiles?format=json` (`mapSize`, optional `region`, `tiles[].{x,y,terrain,water,badwater,entrance,seedling,dead,contaminated,moist,occupants}`) | `timberbot/src/TimberbotReadV2.cs:1184-1258`, `python/tests/fixtures/openapi/tiles.json` | **verified** | Note: real mod emits `terrain`, `water`, `badwater`, and `occupants: [{name, z}]` (NOT `terrainHeight`/`waterDepth`/`contamination`). |
| `GET /api/resources`, `/api/districts`, `/api/population`, `/api/power`, `/api/science`, `/api/wellbeing`, `/api/prefabs`, `/api/alerts`, `/api/notifications`, `/api/distribution`, `/api/trees`, `/api/crops`, `/api/gatherables`, `/api/tree-clusters`, `/api/food-clusters` | `timberbot/src/TimberbotReadV2.cs`, `python/tests/fixtures/openapi/*.json` | **verified** | All 26 golden fixture schemas verified in Rust contract tests. |
| POST endpoints & error prefixes (`not_found`, `invalid_prefab`, `invalid_type`, `invalid_param`, `not_unlocked`, `insufficient_science`, `no_population`, `disabled`, `operation_failed`) | `timberbot/src/TimberbotWrite.cs`, `TimberbotPlacement.cs`, `TimberbotJw.cs:133-156`, `openapi.yaml` | **verified** | Priority accepts 5 levels (`VeryLow`..`VeryHigh`); `ConfigureAutomation` omits `WeatherStation`, `Timer`, and Floodgate On/Off preset heights; **no `/api/save` endpoint exists**. |
| WebSocket `:8086/api/ws` envelope `{"type":"state"|"event"|"pong","payload":...}`, initial `state` push, `heartbeat` request ack, `ping`/`pong`, bearer auth via header or `?token=` | `timberbot/src/TimberbotWebSocketServer.cs`, `docs/websocket-protocol.md` | **verified** | Not ready-gated; broadcasts `drought.approaching`, `drought.start`, `drought.end`, `day.start`, `night.start`, `cycle.start`, `cycle.end`, `building.finished`, `beaver.died`, `character.killed`, `population.changed`, `game.over`. |
| Official built-in HTTP Lever / HTTP Adapter API (`localhost:8080/api/levers`, `switch-on`, `switch-off`, `color`, `adapters`) | [Official Wiki: HTTP Lever](https://timberborn.wiki.gg/wiki/HTTP_Lever) / [HTTP Adapter](https://timberborn.wiki.gg/wiki/HTTP_Adapter), `skills/timberborn-control-http-levers/SKILL.md` | **verified** | Optional mock listener on `:8080` (or ephemeral port in tests); exposed in `timberbot-mcp` as `lever_*` and `adapter_*`. |

---

## 2. Timberborn 1.1 Gameplay Mechanics & Data Table Fidelity

### 2.1 Difficulty Presets (Game Modes)

Source: [Official Timberborn Wiki — Game Mode](https://timberborn.wiki.gg/wiki/Game_Mode) (current 1.0+ preset table; note older pre-1.0 versions had 8 adults and 6-cycle Normal drought handicap).

| Parameter | Easy | Normal | Hard | Status |
|---|---|---|---|---|
| Starting adults / children | 9 / 4 | 9 / 4 | 9 / 4 | **verified** (wiki Game Mode) |
| Food / Water consumption multiplier | 40% (`0.4`) | 100% (`1.0`) | 100% (`1.0`) | **verified** (wiki Game Mode) |
| Starting food / water | 300 / 250 | 130 / 0 | 90 / 0 | **verified** (wiki Game Mode) |
| Temperate duration (days) | 16–19 | 13–17 | 5–8 | **verified** (wiki Game Mode) |
| Drought duration (days) | 2–4 | 5–9 | 15–30 | **verified** (wiki Game Mode) |
| Drought duration handicap | 25% for 8 cycles | 38% for 5 cycles | 20% for 12 cycles | **verified** (wiki Game Mode) |
| Badtide-free initial cycles | 5 | 4 | 3 | **verified** (wiki Game Mode) |
| Badtide chance (after initial cycles) | 40% | 40% | 40% | **verified** (wiki Game Mode) |
| Badtide duration (days) | 1–3 | 4–8 | 15–30 | **verified** (wiki Game Mode) |
| Badtide duration handicap | 30% for 6 cycles | 15% for 5 cycles | 40% for 9 cycles | **verified** (wiki Game Mode) |
| Injury chance / Building demolish refund | 30% / 90% | 100% / 75% | 100% / 75% | **verified** (wiki Game Mode) |

### 2.2 Beaver & Bot Needs, Consumption, and Death Timers

Source: [Official Timberborn Wiki — Needs](https://timberborn.wiki.gg/wiki/Needs).

| Mechanic | Real-Game Value | Status | Mock Notes |
|---|---|---|---|
| Hunger decay & consumption (at 100%) | Range `-300..100`, decay `80/day`, food `+30/unit` → **`2.67 food/day`**; full→death `5.0 d`, starving→death `3.75 d`; `-10` WB, `-50%` work speed | **verified** | Scaled by difficulty food multiplier (`0.4` Easy, `1.0` Normal/Hard). Mock tracks per-colony/beaver need decay and triggers starvation deaths after `3.75` starving days (5.0d from full). |
| Thirst decay & consumption (at 100%) | Range `-300..100`, decay `70/day`, water `+33/unit` → **`2.12 water/day`** (`70/33 ≈ 2.121`, wiki notes `2.13`); full→death `5.71 d`, thirsty→death `4.29 d`; `-10` WB, `-25%` move speed | **verified** | Scaled by difficulty water multiplier. Mock tracks dehydration decay and triggers deaths after `4.29` thirsty days (5.71d from full). |
| Sleep & Shelter | Sleep `-20..80`, decay `60/day` (`+20/h` sheltered, `+15/h` outside); Shelter `-20..80`, decay `30/day`, unsheltered `-3` WB & blocks Folktails breeding | **verified** | Shelter homelessness penalty (`-3` WB) and Folktails breeding gate modeled in mock. |
| Badwater contamination | 3 days Unwell → Contaminated (`-10` WB, `-70%` speed, refuses work); cured by Antidote (`4 d`, `8` units) or Decontamination Pod (`1.23 d`) | **verified** | Mock tracks contamination stress when river contamination exceeds 50% and pumps/beavers are exposed. |
| Bot energy / Biofuel | Range `0..100`, decay `58/day`, Biofuel `+30/unit` → **`1.93 Biofuel/day`**; Catalyst/Punchcards/Grease `+30/unit`, decay `20/day` (`0.67/day`) | **verified** | Data table constant; bot fuel consumption approximated when bots > 0. |
| Beaver carrying capacity | Base lifting capacity **`14`** | **verified** (`python/tests/fixtures/openapi/beavers.json`, wiki Needs History) | Emitted as `liftingCapacity: 14` in mock `/api/beavers`. |

### 2.3 Housing & Reproduction (Folktails & Iron Teeth)

Sources: [Official Wiki — Housing](https://timberborn.wiki.gg/wiki/Housing), [Official Wiki — Breeding Pod](https://timberborn.wiki.gg/wiki/Breeding_Pod), `python/tests/fixtures/openapi/prefabs.json`.

| Building | Faction | Size (X×Y×Z) | Unlock SP | Build Cost | Inhabitants / Output | Status |
|---|---|---|---|---|---|---|
| `Lodge.Folktails` | Folktails | `2×2×1` | 0 | 12 Log | **3** beds; breeds when ≥2 adults & free bed | **verified** (`prefabs.json` + wiki Housing) |
| `MiniLodge.Folktails` | Folktails | `1×2×1` | 50 | 5 Log | **1** bed | **verified** (`prefabs.json` + wiki Housing) |
| `DoubleLodge.Folktails` | Folktails | `2×2×2` | 150 | 20 Log | **6** beds | **verified** (`prefabs.json` + wiki Housing) |
| `TripleLodge.Folktails` | Folktails | `3×2×2` | 250 | 35 Log | **9** beds | **verified** (`prefabs.json` + wiki Housing) |
| `Barrack.IronTeeth` | Iron Teeth | `3×2×2` | 0 | 40 Log | **10** beds (no natural breeding) | **verified** (wiki Housing; exact prefab suffix approximated by convention) |
| `LargeBarrack.IronTeeth` | Iron Teeth | `3×3×2` | 400 | 70 Log | **16** beds | **verified** (wiki Housing) |
| `Rowhouse.IronTeeth` | Iron Teeth | `2×1×3` | 180 | 20 Log | **5** beds | **verified** (wiki Housing) |
| `LargeRowhouse.IronTeeth` | Iron Teeth | `2×1×4` | 600 | 35 Log | **8** beds | **verified** (wiki Housing) |
| `BreedingPod.IronTeeth` | Iron Teeth | `1×1×1` | 0 | 10 Log | **5 Water + 5 Berries / 5 days → 1 kit** | **verified** (wiki Breeding Pod) |
| `AdvancedBreedingPod.IronTeeth` | Iron Teeth | `1×1×2` | 1000 | 5 Gear, 2 MetalBlock | 5 Water + 5 Berries → 1 adult | **verified** (wiki Housing) |

### 2.4 Water Pumping, Tanks, Dams, Floodgates, Evaporation & Irrigation

Sources: [Wiki — Water Pump](https://timberborn.wiki.gg/wiki/Water_Pump), [Deep Water Pump](https://timberborn.wiki.gg/wiki/Deep_Water_Pump), [Tanks](https://timberborn.wiki.gg/wiki/Tanks), [Dam](https://timberborn.wiki.gg/wiki/Dam), [Floodgates](https://timberborn.wiki.gg/wiki/Floodgates), [Evaporation](https://timberborn.wiki.gg/wiki/Evaporation), [Fluids](https://timberborn.wiki.gg/wiki/Fluids), [Irrigation](https://timberborn.wiki.gg/wiki/Irrigation), `prefabs.json`.

| Mechanic / Building | Real-Game Value | Status | Mock Notes |
|---|---|---|---|
| `WaterPump.Folktails` / `DeepWaterPump.IronTeeth` | `2×3×3` (`prefabs.json`), 0 SP, 12 Log, 1 worker, **`0.33 h → 1 Water` (`3 Water/hr`)**, internal storage 15, max depth 2 (FT) / 6 (IT). Reduced efficiency at higher badwater %. | **verified** | Mock scales pump output by `3.0 × effective_work_hours × work_speed_mult × (1 - contamination)`, stopping if reservoir is dry or contamination ≥ 50%. |
| `LargeWaterPump.Folktails` | `3×4×4`, 400 SP, 20 Log + 5 Gear + 10 TreatedPlank; 5× pump rate (`15 Water/hr`) | **verified** (`prefabs.json` + wiki Fluids) | Included in data table. |
| Tanks: `SmallTank` / `MediumTank` / `LargeTank` | Small: `1×1×2`, 0 SP, **15 Log**, cap **30**. Medium: `2×2×3`, 120 SP, **30 Plank + 20 Gear**, cap **300**. Large: `3×3×3`, 600 SP, **80 Plank + 60 Gear + 30 MetalBlock**, cap **1200**. | **verified** (`prefabs.json` + wiki Tanks) | Exact capacities and costs in mock data table. |
| `Dam` | `1×1×1`, 0 SP, 20 Log; blocks water up to **0.65 blocks** spill height | **verified** (`prefabs.json` + wiki Dam) | Exact spill height in data table. |
| `Floodgate` / `DoubleFloodgate` / `TripleFloodgate` | `1×1×3` (150 SP, 10 Log + 5 Plank, max **1.0**), `1×1×4` (250 SP, 20 Log + 10 Plank, max **2.0**), `1×1×5` (500 SP, 30 Log + 15 Plank, max **3.0**). Step `0.05`. Height `0` = wide open; raising blocks/holds water. | **verified** (`prefabs.json` + wiki Floodgates) | Mock clamps height to `[0.0, max_height]` snapped to `0.05`. |
| Time & fluid unit conversion | 1 in-game day = **`460.8 s`** real time (1× speed); `1 m³` fluid = **`5`** drinkable Water units; `1 m³/s = 460.8 m³/day = 2304 units/day` | **verified** (wiki Fluids) | Recorded in data table. |
| Evaporation rate | `0.04608 m³ × evaporation_value` per day; adjusted adjacency score `1..8` → `6.4485`..`1.1600`: **`~0.053 m/day`** for ≥3-wide pools, `0.096 m/day` for 2-wide, `0.200 m/day` for 1-wide | **verified** (wiki Evaporation) | **approximated** in mock: 3-wide reservoir (`0.0534 m/day` when held by raised floodgate; faster drainage when gate is open). |
| Irrigation & Badwater threshold | Max range **`16` blocks** (`2 × adjusted_adjacency_score`) at `0%` pollution, `-6` blocks per +1 elevation; soil stops irrigating and becomes contaminated at **`≥ 50%`** badwater | **verified** (wiki Irrigation + Evaporation + Fluids) | **approximated** in mock: river-bank moisture scales with reservoir depth & `< 0.50` contamination. |

### 2.5 Crops, Food Processing, Wood, Science & Power

Sources: [Wiki — Crops](https://timberborn.wiki.gg/wiki/Crops), [Farmhouse](https://timberborn.wiki.gg/wiki/Farmhouse), [Fermenter](https://timberborn.wiki.gg/wiki/Fermenter), [Lumber Mill](https://timberborn.wiki.gg/wiki/Lumber_Mill), [Inventor](https://timberborn.wiki.gg/wiki/Inventor), [Power](https://timberborn.wiki.gg/wiki/Power), [Geothermal Engine](https://timberborn.wiki.gg/wiki/Geothermal_Engine), [Aquifer Drill](https://timberborn.wiki.gg/wiki/Aquifer_Drill), `prefabs.json`, `0xguy07/timberborn-tools`.

| Building / Crop | Real-Game Value | Status | Mock Notes |
|---|---|---|---|
| `EfficientFarmHouse.Folktails` | `2×3×2`, 0 SP, **25 Log**, 2 workers (Folktails). Note capital `H` in real prefab name! | **verified** (`prefabs.json`) | Mock accepts both `EfficientFarmHouse.Folktails` and legacy alias `Farmhouse.Folktails` for backwards compatibility. |
| `FarmHouse.IronTeeth` / `Farmhouse.IronTeeth` | `2×2×2`, 0 SP (unlockable 500? wiki shows 20 Log, 2 workers; Iron Teeth crop set: Kohlrabi, Cassava, Soybean, Canola, Corn, Eggplant) | **verified** (wiki Farmhouse; exact prefab casing approximated) | Supported in mock for Iron Teeth scenarios. |
| Folktails crops | Carrot: **4 d, yield 3** (`0.75/d`); Sunflower: **5 d, yield 2** (`0.40/d`); Potato: **6 d, yield 1** (4 grilled = `0.67/d`); Wheat: **10 d, yield 3**; Cattail: **8 d, yield 3**; Spadderdock: **12 d, yield 3**. Beehive `-30%` maturity. | **verified** (wiki Crops) | Exact maturity days & yields in mock data table. |
| Iron Teeth crops | Kohlrabi: **3 d, yield 2** (`0.67/d`, edible raw); Cassava: **5 d, yield 1** (`0.50/d` fermented); Soybean: **8 d, yield 2**; Canola: **9 d, yield 3**; Corn: **10 d, yield 2**; Eggplant: **12 d, yield 3**. | **verified** (wiki Crops) | Exact maturity days & yields in mock data table. |
| `Fermenter.IronTeeth` | `1×3×3`, 25 Log, 50 hp, 1 worker: `4 Cassava → 10 FermentedCassava` (`2 h`), `6 Soybean + 1 CanolaOil → 20 FermentedSoybeans` (`3 h`), `4 Mushroom → 16 FermentedMushrooms` (`2 h`) | **verified** (wiki Fermenter) | Exact recipe ratios in data table; mock simulates Fermenter when staffed & powered. |
| Trees & `LumberMill` | Cut yields: Birch **1 Log**, Pine **2 Logs**, Oak **8 Logs** (`0xguy07/timberborn-tools`). `LumberMill`: `2×3×3`, 0 SP, 15 Log, 50 hp, 1 worker, **`1 Log → 1 Plank` (`1.3 h`)**. `Forester`: `2×2×4`, 30 SP, `10 Log + 7 Plank`. | **verified** (`prefabs.json`, wiki Lumber Mill, `0xguy07/timberborn-tools`) | Modeled in mock data table & wood production. |
| `Inventor` | `2×2×3`, 0 SP, 12 Log, 1 beaver-only worker, **`1 SP/hr`** | **verified** (`prefabs.json` + wiki Inventor) | Mock produces `1.0 × work_hours × work_speed_mult` SP/day per staffed Inventor. |
| Power generators & consumers | Power Wheel `50 hp` (35–85), Large Power Wheel `300 hp` (4 beavers, max 510), Water Wheel `270 hp/(m³/s)`, Compact WW `120`, Large WW `540`, Wind Turbine `150 peak / ~68 avg` (120 SP), Large Wind Turbine `300 peak / ~144 avg` (1400 SP), Steam Engine `400 hp` (400 SP), Geothermal Engine **`400 hp`** (160 SP, 50 Log + 30 Plank, unmanned), Aquifer Drill **`400 hp` consumer** (400 SP, 40 Plank + 25 Gear + 15 MetalBlock), Compact Mechanical Pump **`700 hp`** (1.1), (Deep) Mechanical Pump **`500 hp`** (1.1 rebalance). | **verified** (wiki Power, Geothermal Engine, Aquifer Drill, Update 1.1 patch notes, `prefabs.json`) | Mock tracks power network `supply` vs `demand` and slows powered workshops proportionally when `supply < demand` (and halts when `supply == 0`). |

### 2.6 Wellbeing, Monuments & Wonders

Sources: `python/tests/fixtures/openapi/wellbeing.json`, [Wiki — Wonders](https://timberborn.wiki.gg/wiki/Wonders), [Earth Recultivator](https://timberborn.wiki.gg/wiki/Earth_Recultivator), [Earth Repopulator](https://timberborn.wiki.gg/wiki/Earth_Repopulator), Update 1.1 patch notes (`steamdb.info/patchnotes/25096761/`).

| Item | Real-Game Value | Status |
|---|---|---|
| Folktails pre-1.1-attractions wellbeing max | **`78`** (`BasicNeeds 5 + Nutrition 16 + SocialLife 11 + Fun 11 + Aesthetics 9 + Awe 26`) | **verified** (`python/tests/fixtures/openapi/wellbeing.json`) |
| Wellbeing work-speed / movement / lifespan curve | WB 5–6 `+20%` work speed; WB 15 unlocks Iron Teeth (pre-1.1); WB 67–71 `+260%` work speed, `+120%` life expectancy, `+70%` movement speed | **verified** (`0xguy07/timberborn-tools` citing wiki) — Iron Teeth unlock on 1.1 remains **unverified** |
| Wonders (`EarthRecultivator` / `EarthRepopulator`) | `7×7×8` / `7×7×10`, 20,000 SP, 4 construction stages (25% each), **`+10` Awe in `40`-tile radius**. Launch: `500 Paper + 500 Extract` (FT) / `500 Berries + 500 TreatedPlank + up to 8 pilots` (IT). | **verified** (wiki Wonders, Earth Recultivator, Earth Repopulator, `prefabs.json`) |
| 1.1 Tier 4 Monuments (`Hall of Abundance`, `Arch of Progress`) & 1.1 Attractions (`Sauna`, `Domed Garden`, `Massager`, `Dance Pit`) | Unlock SP & build costs verified from 1.1 patch notes; **exact awe/wellbeing bonus points, radius, and new post-1.1 max wellbeing are NOT published in the patch notes** | **guess / unverified** |
| `Numbercruncher` (Iron Teeth) | Power demand **`400 hp`** verified (wiki Power); exact unlock SP and science output/hr not confirmed from primary source | **partial** (`400 hp` verified; SP/rate **guess / unverified**) |
| Latest Steam/GOG Timberborn build (`> 1.1.2.0`) | Manifest pins `MinimumGameVersion: 1.1.2.0`; live compatibility with latest Steam/GOG build not yet tested | **guess / unverified** |

### 2.7 `.timber` Save / Map Archive Format

Sources: `0xguy07/timberborn-tools` (`bridge/tb_save.py`, `docs/timber-map-format.md`, verified on 1.0.13.1 & 1.1.2.4; MIT license).

| Element | Real-Game Format | Status | `timberbot-save` Support |
|---|---|---|---|
| ZIP members | `world.json`, `version.txt`, `save_metadata.json` (saves) or `map_metadata.json` (maps), thumbnail `.jpg` | **verified** (`tb_save.py`, `Practice Valley.timber`) | Reads `world.json` + optional `version.txt` & metadata. |
| Terrain voxels (1.0/1.1) | `Singletons.TerrainMap.Voxels.Array`: space-separated `0`/`1`, 23 vertical layers (`z * X * Y + y * X + x`). Legacy/synthetic saves may use `TerrainMap.Heights.Array`. | **verified** (`tb_save.py`, `Practice Valley.timber`) | Supports **both** `Voxels.Array` (23 layers) and `Heights.Array`. |
| Water columns (1.0/1.1) | `Singletons.WaterMapNew.WaterColumns.Array`: `Levels * X * Y` entries of `"0"` or `"depth:contamination:?:floorZ:prev"`. | **verified** (`tb_save.py`) | Parses both colon-delimited 1.1 water columns and plain float arrays. |
| Entity inventories | `Components["Inventory:*"].Storage.Goods[]` with `Good` (either string ID in 1.0/1.1 or `{"Id": ...}` object) + `Amount` (excludes `Inventory:ConstructionSite`) | **verified** (`tb_save.py`) | Supports both string `Good` and `{"Id": ...}` forms, skipping `Inventory:ConstructionSite`. |

---

## 3. Intentional Approximations in `timberbot-mock`

To keep `cargo test --workspace` deterministic and fast (~1 s without Unity or game DLLs), `timberbot-mock` deliberately approximates:

1. **Fluid simulation**: Models a 1D river channel + reservoir depth (`reservoir_water_depth`) and contamination (`river_contamination`) using verified wiki evaporation rates (`0.0534 m/day` for a ≥3-wide dammed reservoir) and dam/floodgate retention, rather than solving 3D voxel fluid flow.
2. **Pathfinding & building placement**: Checks map bounds, river tiles, shore adjacency for pumps, tile occupancy, unlock status, and material costs, rather than full 3D `BlockObjectSpec` entrance offsets and A* terrain navigation.
3. **Commute & hauling latency**: Applies a verified workday efficiency factor (`0.85` effective productive time of shift hours, **approximated**) instead of per-beaver agent movement ticks.
4. **Reproduction cadence**: Folktails kits spawn when free beds > 0, ≥2 adults exist, and no hunger/thirst crisis; Iron Teeth kits spawn from staffed/supplied `BreedingPod` cycles (`5 Water + 5 Berries / 5 days` verified, exact pod scheduling approximated).
5. **Wellbeing bonuses**: Approximates colony work-speed multiplier from average wellbeing using the verified wiki curve anchors (`WB 0 -> 1.0x`, `WB 5 -> 1.2x`, `WB 15 -> 1.6x`, `WB 70 -> 3.6x`).
