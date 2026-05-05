use crate::{FrameEventArgs, Timer};

pub struct TimerManager {
    timers: Vec<Timer>,
}

impl TimerManager {
    pub fn new() -> Self {
        Self { timers: Vec::new() }
    }

    pub fn add_timer(&mut self, timer: Timer) {
        self.timers.push(timer);
    }

    pub fn update_timers(&mut self, frame_event_args: FrameEventArgs) {
        for timer in &mut self.timers {
            timer.update(frame_event_args.delta_seconds);
        }
        self.timers.retain(|timer| timer.is_active);
    }

    pub fn len(&self) -> usize {
        self.timers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::TimerManager;
    use crate::{FrameEventArgs, Timer};
    use std::sync::{Arc, Mutex};

    #[test]
    fn timer_manager_removes_inactive_timers() {
        let fired = Arc::new(Mutex::new(0));
        let fired_clone = fired.clone();
        let mut manager = TimerManager::new();
        manager.add_timer(Timer::new(10, false, move || {
            *fired_clone.lock().unwrap() += 1;
        }));
        manager.update_timers(FrameEventArgs::new(0.01));
        assert_eq!(*fired.lock().unwrap(), 1);
        assert!(manager.is_empty());
    }
}
