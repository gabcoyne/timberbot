# Reaching the mod from another machine

The mod binds to loopback unless you opt in. A Grok Bot running on a different machine should reach the game through an SSH tunnel. That leaves the mod on the defaults upstream already ships, so later merges stay boring.

## Verified defaults

From `timberbot/src/settings.json`, `TimberbotService`, and `TimberbotPure`:

- `listenAddress` defaults to `127.0.0.1`. `httpPort` defaults to `8085`. `wsPort` defaults to `8086`.
- `authToken` defaults to empty. Empty means no `Authorization` check.
- If `listenAddress` is not a loopback name (`localhost`, `127.0.0.1`, `::1`) and `authToken` is empty, **neither listener starts**. The guard is `TimberbotPure.RequiresAuthToken`.
- When `authToken` is set, every `/api/*` request except `GET /api/ping` needs `Authorization: Bearer <token>`. The WebSocket upgrade needs the same token, in that header or in `?token=`. Comparison is constant-time.
- The ready gate is separate. Until Launch, non-exempt `/api/*` routes return `409` with `{"error":"game_not_ready","hint":"player must press Launch in the Timberbot widget"}`. Exempt paths are `/api/ping`, `/api/ready`, and `/api/agent/*`. The WebSocket is not ready-gated.

`GET /api/ping` is unauthenticated on purpose, so a client can probe liveness before it knows a token is required. The C# handler currently returns `"ready": true` in that body even when the gate is closed (`TimberbotHttpServer`). Use `GET /api/agent/state` for the real gate.

## Preferred: SSH tunnel

Keep `listenAddress` at `127.0.0.1` and `authToken` empty on the game machine. From the bot machine:

```bash
ssh -N \
  -L 8085:127.0.0.1:8085 \
  -L 8086:127.0.0.1:8086 \
  game-user@game-host
```

Point the bot at `http://127.0.0.1:8085` and `ws://127.0.0.1:8086/api/ws` on its own side of the tunnel.

Why this is the default recommendation:

- The mod process never listens on a LAN or public interface.
- You do not have to set `authToken`, so a stock `settings.json` still matches upstream.
- Stopping the SSH session closes the path. The game does not stay reachable.

Use a dedicated SSH user and key. Do not put the game host's credentials in the toolkit, the skills, or a bot prompt.

If you also run `tbot serve`, its MCP listener is a third loopback port (`127.0.0.1:8091` by default). Tunnel that port too. A later Rust toolkit in this repo can speak MCP on the bot side and use the tunnelled 8085/8086 ports. That process is not a route on the mod. There is no `/mcp` path on the HTTP server.

The official HTTP Lever / HTTP Adapter API is a different listener, `127.0.0.1:8080`, built into the game. Tunnel `8080` as well if the bot will use levers. That API is not implemented by this mod.

## Opt-in: token-protected non-loopback bind

The mod already supports a non-loopback bind. It is off unless you edit `settings.json` on the game machine. Do not change the committed default.

```json
{
  "listenAddress": "0.0.0.0",
  "authToken": "replace-with-a-long-random-secret",
  "httpPort": 8085,
  "wsPort": 8086
}
```

`0.0.0.0`, `+`, `*`, and a LAN address all count as non-loopback. The mod refuses to start if `authToken` is missing or blank. Clients then send `Authorization: Bearer <token>` (Python: `TBOT_AUTH_TOKEN`, `[client].auth_token`, or `tbot --auth-token`).

Reload the save after changing `settings.json`. The widget copy says the same thing: a non-loopback address requires a token.

Prefer the SSH tunnel over this bind. A token-protected socket is still a network service on the game machine. The committed `settings.json` stays on loopback with an empty token so a fresh install is not reachable from the network.

## Client overrides

The Python client never reads the mod's `settings.json`. Point it at the tunnel (or the bound address) with `--host` / `--port`, `TBOT_HOST` / `TBOT_PORT` / `TBOT_WS_PORT`, or `[client]` in `config.toml`. See [Getting Started](getting-started.md#remote-connections).
