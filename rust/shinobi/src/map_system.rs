use crate::ClientEntityManager;
use sekai::{GameStateMapData, GridId};

#[derive(Debug, Clone, Default)]
pub struct MapSystem;

impl MapSystem {
    pub fn new() -> Self {
        Self
    }

    pub fn apply_game_state(&self, entities: &mut ClientEntityManager, map_data: &GameStateMapData) {
        entities.apply_map_data(map_data);
    }

    pub fn grid_exists(&self, entities: &ClientEntityManager, grid_id: GridId) -> bool {
        entities
            .inner
            .map_grid_components
            .values()
            .any(|component| component.grid_index == grid_id)
    }
}

#[cfg(test)]
mod tests {
    use super::MapSystem;
    use crate::ClientEntityManager;
    use keisan::{Angle, Vector2, Vector2i};
    use sekai::{ChunkDatum, GameStateMapData, GridDatum, GridId, MapCoordinates, MapId, Tile, TileRenderFlag};

    #[test]
    fn map_system_applies_grid_state() {
        let mut entities = ClientEntityManager::new();
        let system = MapSystem::new();
        system.apply_game_state(
            &mut entities,
            &GameStateMapData {
                grid_data: std::iter::once((
                    GridId::new(4),
                    GridDatum {
                        coordinates: MapCoordinates::new(Vector2::ZERO, MapId::new(1)),
                        angle: Angle::ZERO,
                        chunk_data: vec![ChunkDatum::create_modified(
                            Vector2i::new(0, 0),
                            vec![Tile::new(2, TileRenderFlag(0), 0)],
                        )],
                    },
                ))
                .collect(),
            },
        );
        assert!(system.grid_exists(&entities, GridId::new(4)));
    }
}
