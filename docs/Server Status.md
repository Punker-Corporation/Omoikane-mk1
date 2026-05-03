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

A production status endpoint should be a thin projection of server state, not a
second source of truth. The stable JSON shape should start small:

```json
{
  "name": "Omoikane",
  "players": 0,
  "tick": 0,
  "tick_rate": 60,
  "build": "0.1.0"
}
```

Transport is deliberately not fixed here. The implementation can be HTTP,
QUIC-side metadata or an in-process host API as long as it reads from the same
authoritative state and never mutates simulation data.
