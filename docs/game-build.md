# Pinned Timberborn build

This page records the game build the mod is actually pinned to, using only
what is in this repository. It does not claim compatibility with the newest
Steam or GOG build.

## What is verified

| Fact | Where it is verified |
|---|---|
| Declared minimum game version is **1.1.2.0** | `timberbot/src/manifest.json` field `MinimumGameVersion` |
| Mod assembly version is **0.8.0** | `manifest.json` `Version` and `Timberbot.csproj` `<Version>` |
| The mod targets **.NET Standard 2.1** (C# 9) | `Timberbot.csproj` `TargetFramework` and `LangVersion` |
| Game assemblies are referenced by file name from `$(GameManagedDir)` with `Publicize="true"` and `<Private>false</Private>`. No game DLL version, file hash, or Unity editor pin is stored in the project | `Timberbot.csproj` |
| Default `GameManagedDir` paths exist for Windows, macOS, and Linux. They are install locations, not build numbers | `Timberbot.csproj`; macOS path also in `docs/devenv.md` |
| Builds compiled against 1.1 DLLs do not load on 1.0.x, because 1.1 changed `IBlockObjectPlacer.Place`, `Relay` inputs, and `GoodCarrier` | `AGENTS.md` |
| `docs/devenv.md` requires a .NET SDK **≥ 8** to build and test. The test project is `net8.0` and does not need game DLLs | `docs/devenv.md`, `timberbot/test/Timberbot.Tests.csproj` |
| `AGENTS.md` says "Timberborn uses Unity Engine 6000.3.6f1 as of 1.0" | That sentence is in-repo. It dates the note to **1.0**. It is not a pin of the 1.1.2 game, and it does not say which Unity version ships in 1.1.2.0 |

`dotnet build` of `timberbot/src/Timberbot.csproj` needs the game's `Managed/` folder on the machine. CI in this fork skips that build when those DLLs are absent. The pure xUnit project under `timberbot/test/` does not need them.

## What this pin is

The only version this repository pins is the **manifest minimum, 1.1.2.0**.

That means:

- The mod declares it will not load on older games.
- The authors ported the mod to the 1.1 placement, relay, and carrier APIs (see the 1.1.2 commits and `AGENTS.md`).
- Nothing in `manifest.json`, `Timberbot.csproj`, `docs/devenv.md`, or the Publicize references records a patch build newer than 1.1.2.0, a Steam build id, or a "latest" channel.

## What is not verified

**Compatibility with the latest Timberborn build is unverified.** Do not treat this tree as tested against current Steam or GOG.

In particular, this repository does **not** prove:

- The current Steam or experimental build number (community notes that mention 1.1.2.4, 1.1.2.7, or later patches are outside this repo).
- That 1.1 moved the engine to Unity 6000.5, or any Unity version for 1.1.
- That `MinimumGameVersion` 1.1.2.0 still loads on whatever build is current when you read this.
- A checksum of the DLLs the Publicize references were last compiled against.

When you build on a machine that has the game, record the build you actually compiled against (the game's version string and the `Managed/` path) next to the binary you deploy. Rebuild after every game update. A successful compile against one 1.1.2.x install does not by itself extend this pin.

## Agent-facing gaps checked while writing this pin

These were checked against `openapi.yaml` and the C# route tables. They are contract facts, not game-balance facts.

| Need | Result |
|---|---|
| Set a floodgate's current height | Present. `POST /api/building/floodgate` with `{"id","height"}`. Implemented by `TimberbotWrite.SetFloodgateHeight`, which clamps to `0..Floodgate.MaxHeight` and calls `SetHeightAndSynchronize`. |
| Separate badtide flag on `GET /api/weather` | Not in the OpenAPI `Weather` schema. `isHazardous` is documented as drought or badtide together (`docs/api-reference.md`). |
| Trigger a save over HTTP | **Absent.** No `/api/save` route in `openapi.yaml` or `TimberbotHttpServer`. `TimberbotAutoLoad` can load a save at the main menu (`autoload.json` or `--tb-settlement` / `--tb-save`). The mod publishes an `autosave` WebSocket event when the game autosaves. An agent cannot create a checkpoint through the API. |
| `POST /mcp` on the mod | **Absent.** MCP in the Python client is `tbot serve` on `127.0.0.1:8091` (`/sse`), not a route on the mod. |

No HTTP or WebSocket route was added or renamed for this pin. The public contract stays aligned with upstream.
