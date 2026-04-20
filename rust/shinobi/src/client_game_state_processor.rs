use sekai::GameState;

#[derive(Debug, Clone)]
pub struct ClientGameStateProcessor {
    state_buffer: Vec<GameState>,
    last_full_state: Option<GameState>,
    waiting_for_full: bool,
    interpolation: bool,
    interp_ratio: usize,
}

impl Default for ClientGameStateProcessor {
    fn default() -> Self {
        Self {
            state_buffer: Vec::new(),
            last_full_state: None,
            waiting_for_full: true,
            interpolation: true,
            interp_ratio: 0,
        }
    }
}

impl ClientGameStateProcessor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_buffer_size(&self) -> usize {
        if self.interpolation { 3 } else { 2 }
    }

    pub fn target_buffer_size(&self) -> usize {
        self.min_buffer_size() + self.interp_ratio
    }

    pub fn add_new_state(&mut self, state: GameState) {
        if state.from_sequence == jikan::GameTick::ZERO {
            if self
                .last_full_state
                .as_ref()
                .map(|full| full.to_sequence < state.to_sequence)
                .unwrap_or(true)
            {
                self.last_full_state = Some(state);
                return;
            }
        }

        if self
            .state_buffer
            .iter()
            .any(|existing| existing.to_sequence == state.to_sequence)
        {
            return;
        }

        self.state_buffer.push(state);
        self.state_buffer.sort_by_key(|state| state.to_sequence.value);
    }

    pub fn pop_next_state(&mut self) -> Option<GameState> {
        if self.waiting_for_full {
            let full = self.last_full_state.clone()?;
            if self.state_buffer.is_empty() {
                self.waiting_for_full = false;
                return Some(full);
            }
            if self.state_buffer.len() < self.target_buffer_size().saturating_sub(1) {
                return None;
            }
            self.waiting_for_full = false;
            return Some(full);
        }

        if self.state_buffer.is_empty() {
            None
        } else {
            Some(self.state_buffer.remove(0))
        }
    }

    pub fn reset(&mut self) {
        self.state_buffer.clear();
        self.last_full_state = None;
        self.waiting_for_full = true;
    }
}

#[cfg(test)]
mod tests {
    use super::ClientGameStateProcessor;
    use jikan::GameTick;
    use sekai::GameState;

    #[test]
    fn processor_waits_for_full_then_emits_states() {
        let mut processor = ClientGameStateProcessor::new();
        processor.add_new_state(GameState {
            from_sequence: GameTick::ZERO,
            to_sequence: GameTick::new(3),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        });
        processor.add_new_state(GameState {
            from_sequence: GameTick::new(3),
            to_sequence: GameTick::new(4),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        });
        processor.add_new_state(GameState {
            from_sequence: GameTick::new(4),
            to_sequence: GameTick::new(5),
            last_processed_input: 0,
            entity_states: Vec::new(),
            player_states: Vec::new(),
            entity_deletions: Vec::new(),
            map_data: None,
            extrapolated: false,
            payload_size: 0,
        });
        assert_eq!(processor.pop_next_state().unwrap().to_sequence, GameTick::new(3));
        assert_eq!(processor.pop_next_state().unwrap().to_sequence, GameTick::new(4));
    }
}
