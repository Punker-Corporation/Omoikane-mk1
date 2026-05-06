use crate::Component;
use jikan::Timer;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub struct TimerHandle {
    cancelled: Arc<AtomicBool>,
}

impl TimerHandle {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

struct TimerRegistration {
    timer: Timer,
    cancelled: Arc<AtomicBool>,
}

pub struct TimerComponent {
    pub base: Component,
    timers: Vec<TimerRegistration>,
    pub remove_on_empty: bool,
}

impl Default for TimerComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerComponent {
    pub fn new() -> Self {
        Self {
            base: Component::new("TimerComponent"),
            timers: Vec::new(),
            remove_on_empty: true,
        }
    }

    pub fn timer_count(&self) -> usize {
        self.timers.len()
    }

    pub fn update(&mut self, frame_time: f32) {
        for registration in &mut self.timers {
            if registration.cancelled.load(Ordering::SeqCst) {
                continue;
            }
            registration.timer.update(frame_time);
        }

        self.timers
            .retain(|timer| timer.timer.is_active && !timer.cancelled.load(Ordering::SeqCst));
    }

    pub fn add_timer(&mut self, timer: Timer) -> TimerHandle {
        let cancelled = Arc::new(AtomicBool::new(false));
        self.timers.push(TimerRegistration {
            timer,
            cancelled: cancelled.clone(),
        });
        TimerHandle { cancelled }
    }

    pub fn delay(
        &mut self,
        milliseconds: i32,
        on_fired: impl FnMut() + Send + Sync + 'static,
    ) -> TimerHandle {
        self.spawn(milliseconds, on_fired)
    }

    pub fn spawn(
        &mut self,
        milliseconds: i32,
        on_fired: impl FnMut() + Send + Sync + 'static,
    ) -> TimerHandle {
        self.add_timer(Timer::new(milliseconds, false, on_fired))
    }

    pub fn spawn_repeating(
        &mut self,
        milliseconds: i32,
        on_fired: impl FnMut() + Send + Sync + 'static,
    ) -> TimerHandle {
        self.add_timer(Timer::new(milliseconds, true, on_fired))
    }
}

#[cfg(test)]
mod tests {
    use super::TimerComponent;
    use std::sync::{Arc, Mutex};

    #[test]
    fn timer_component_runs_and_removes_finished_timers() {
        let fired = Arc::new(Mutex::new(0));
        let mut component = TimerComponent::new();
        let fired_clone = fired.clone();
        component.spawn(20, move || {
            *fired_clone.lock().unwrap() += 1;
        });
        component.update(0.01);
        assert_eq!(*fired.lock().unwrap(), 0);
        component.update(0.02);
        assert_eq!(*fired.lock().unwrap(), 1);
        assert_eq!(component.timer_count(), 0);
    }
}
