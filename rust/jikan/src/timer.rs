pub struct Timer {
    time_counter_ms: i32,
    pub time_ms: i32,
    pub is_repeating: bool,
    pub is_active: bool,
    on_fired: Box<dyn FnMut() + Send + Sync + 'static>,
}

impl Timer {
    pub fn new(
        milliseconds: i32,
        is_repeating: bool,
        on_fired: impl FnMut() + Send + Sync + 'static,
    ) -> Self {
        Self {
            time_counter_ms: milliseconds,
            time_ms: milliseconds,
            is_repeating,
            is_active: true,
            on_fired: Box::new(on_fired),
        }
    }

    pub fn update(&mut self, frame_time: f32) {
        if !self.is_active {
            return;
        }

        self.time_counter_ms -= (frame_time * 1000.0) as i32;
        if self.time_counter_ms <= 0 {
            (self.on_fired)();
            if self.is_repeating {
                self.time_counter_ms += self.time_ms;
            } else {
                self.is_active = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Timer;
    use std::sync::{Arc, Mutex};

    #[test]
    fn one_shot_timer_fires_once_and_deactivates() {
        let fired = Arc::new(Mutex::new(0));
        let fired_clone = fired.clone();
        let mut timer = Timer::new(50, false, move || {
            *fired_clone.lock().unwrap() += 1;
        });
        timer.update(0.025);
        assert_eq!(*fired.lock().unwrap(), 0);
        assert!(timer.is_active);
        timer.update(0.025);
        assert_eq!(*fired.lock().unwrap(), 1);
        assert!(!timer.is_active);
    }
}
