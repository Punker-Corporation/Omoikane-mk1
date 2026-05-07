# Omoikane mk1

Omoikane is a Rust-first experimental engine and server runtime descended from
the Robust Toolbox architecture, but no longer organized as a C#/.NET fork. The
workspace is now rooted directly in Rust crates and keeps the runtime surface
self-contained inside this repository.

The current crates are:

- `keisan`: deterministic math primitives, colors, vectors, matrices and geometry.
- `jikan`: ticks, timers, frame timing and loop support.
- `butsuri`: collision, broadphase, bodies, fixtures, joints and ray queries.
- `sekai`: shared ECS, maps, transforms, serialization, physics state and game state.
- `daikoku`: authoritative server systems, networking queues, PVS and prediction inputs.
- `shinobi`: client-side state application, interpolation, prediction and local systems.
- `hikari`: CPU-validated render graph, resource catalog, frame submissions, render/compute command lists, structural graphics handles and future graphics runtime boundary.
- `omoikane_app`: headless app host for local server/client orchestration, sandbox entity spawning, CPU render extract and future vertical slices.
- `omoikane_web`: Actix Web integration, server status routes and RouterOS rack profile generation.
- `xtask`: repository maintenance checks for the Rust-only layout.

## Commands

```bash
cargo run -p xtask -- architecture-map
cargo run -p xtask -- verify-architecture
cargo run -p xtask -- verify-layout
cargo run -p omoikane_app --example headless_sandbox
cargo test -p omoikane_web
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Direction

Omoikane favors deterministic data flow, explicit runtime state, server
authority, client prediction, serializable component payloads and testable
systems. The migration target is not a line-by-line port: new work should keep
the behavior compatible where useful while replacing legacy assumptions with
idiomatic Rust APIs, better invariants and measurable performance wins.

See `docs/Omoikane Roadmap.md` for the long-term implementation roadmap and
continuity prompt, `docs/Web Server Architecture.md` for the Actix/RouterOS
web-server path, and `docs/Research Intake.md` for the current technical
intake of papers, theses, talks and Rust repositories being evaluated for
future Omoikane subsystems.

## Legal

See `legal.md` and the license files in this repository.
