# Omoikane Map Format

Omoikane maps are deterministic snapshots of world topology, tile data and
entity state. The Rust runtime models this through `sekai::MapManager`,
`sekai::MapGrid`, `sekai::MapChunk`, `sekai::Tile`, transform components and
serialized component payloads.

The current runtime representation is optimized for simulation correctness:
maps own grids, grids own sparse chunks, chunks own packed tile arrays, and
entities reference map or grid space through explicit `MapId`, `GridId` and
`EntityUid` values.

## Sections

### `meta`

Stores format and provenance data.

- `format`: integer format version.
- `name`: optional human-readable map name.
- `author`: optional author or generator identity.
- `postmapinit`: whether generation and initialization have already run.

### `tilemap`

Maps stable tile definition names to compact numeric tile ids used inside
chunk tile arrays. Runtime code should never assume that numeric tile ids are
stable across content packs.

### `grids`

Stores one or more grid records. A grid record contains:

- `settings`: tile size, chunk size and snap size.
- `chunks`: sparse chunk records keyed by chunk coordinates.
- grid entity state, when the grid participates in ECS state.

### `entities`

Stores serialized entities and component payloads. Entity references use
`EntityUid`, and grid references use `GridId`; absent or external references
must be represented explicitly instead of silently remapped.

## Binary Tile Data

Chunk tile data is packed in row-major order. Each tile currently carries a
compact tile id and render/metadata flags. Runtime code should prefer the
typed `Tile` API over parsing raw bytes directly.

## Direction

The old YAML-heavy shape is acceptable as an interchange format, but the engine
path should prefer binary or hybrid encodings once the serializer boundary is
complete. The target is deterministic loading, lossless round-tripping,
content-addressed chunks and cheap incremental replication.
