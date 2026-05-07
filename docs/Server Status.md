# Omoikane Server Status

The authoritative server crate is `daikoku`. Its network-facing state is built
around explicit session records, outbound message queues, player state deltas,
visibility filtering and acknowledged game-state ticks.

## Runtime State

Server state is intentionally split into small systems:

- `DaikokuServer`: orchestration, tick lifecycle and message pumping.
- `ServerNetManager`: inbound/outbound protocol queues.
- `PlayerManager`: connection and session state.
- `ServerGameStateManager`: full and incremental snapshots.
- `PvsSystem`: per-player visibility sets.
- `PhysicsSystem`, `MapSystem`, `TransformSystem`: authoritative simulation.

## Status Contract

A production status endpoint is a thin projection of server state, not a second
source of truth. `DaikokuServer::status_snapshot` reads the authoritative
runtime state and `HttpStatusService` turns that snapshot into a small HTTP/1
response without pulling an async runtime or external web framework into
`daikoku`.

The first supported routes are:

- `GET /status` and `HEAD /status`: full status JSON.
- `GET /status.json` and `HEAD /status.json`: same payload, explicit suffix.
- `GET /health` and `GET /healthz`: minimal health JSON.

The stable JSON shape starts small:

```json
{
  "name": "Omoikane",
  "state": "running",
  "players": 0,
  "max_players": 32,
  "tick": 0,
  "tick_rate": 60,
  "queues": {
    "sessions": 0,
    "outbound_messages": 0,
    "queued_inputs": 0,
    "queued_entities": 0,
    "queued_player_list_requests": 0
  }
}
```

Transport is deliberately still thin here. The implementation can be wrapped by
TCP, QUIC-side metadata or an in-process host API as long as it reads from the
same authoritative state and never mutates simulation data. This keeps the
server path compatible with a future Omoikane-native runtime that can be
measured directly against existing Rust web stacks.
