#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityLifeStage {
    PreInit = 0,
    Initializing,
    Initialized,
    MapInitialized,
    Terminating,
    Deleted,
}
