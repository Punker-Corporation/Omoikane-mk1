use std::time::{Duration, Instant};

pub trait Stopwatch {
    fn elapsed(&self) -> Duration;
    fn restart(&mut self);
    fn start(&mut self);
}

#[derive(Debug, Clone)]
pub struct RealStopwatch {
    start: Option<Instant>,
}

impl Default for RealStopwatch {
    fn default() -> Self {
        let mut timer = Self { start: None };
        timer.start();
        timer
    }
}

impl Stopwatch for RealStopwatch {
    fn elapsed(&self) -> Duration {
        self.start.map(|x| x.elapsed()).unwrap_or(Duration::ZERO)
    }

    fn restart(&mut self) {
        self.start = Some(Instant::now());
    }

    fn start(&mut self) {
        if self.start.is_none() {
            self.start = Some(Instant::now());
        }
    }
}
