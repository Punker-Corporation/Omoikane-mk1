use crate::{GameTick, Stopwatch};
use std::time::Duration;

const NUM_FRAMES: usize = 60;

pub struct GameTiming<T: Stopwatch> {
    real_timer: T,
    real_frame_times: [i64; NUM_FRAMES],
    frame_idx: usize,
    last_real_time: Duration,
    cached_cur_time: (Duration, GameTick),
    pub in_simulation: bool,
    pub paused: bool,
    pub cur_tick: GameTick,
    pub last_tick: Duration,
    pub tick_rate: u8,
    pub tick_remainder: Duration,
    pub cur_frame: u32,
    pub tick_timing_adjustment: f32,
    pub last_real_tick: GameTick,
    pub is_first_time_predicted: bool,
}

impl<T: Stopwatch + Default> Default for GameTiming<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Stopwatch> GameTiming<T> {
    pub fn new(mut real_timer: T) -> Self {
        real_timer.start();
        Self {
            real_timer,
            real_frame_times: [0; NUM_FRAMES],
            frame_idx: 0,
            last_real_time: Duration::ZERO,
            cached_cur_time: (Duration::ZERO, GameTick::FIRST),
            in_simulation: false,
            paused: true,
            cur_tick: GameTick::FIRST,
            last_tick: Duration::ZERO,
            tick_rate: NUM_FRAMES as u8,
            tick_remainder: Duration::ZERO,
            cur_frame: 1,
            tick_timing_adjustment: 0.0,
            last_real_tick: GameTick::ZERO,
            is_first_time_predicted: true,
        }
    }

    pub fn cur_time(&self) -> Duration {
        let (cached_time, last_time_tick) = self.cached_cur_time;
        let mut time = cached_time;
        if self.cur_tick.value >= last_time_tick.value {
            time += self.tick_period().mul_f64((self.cur_tick.value - last_time_tick.value) as f64);
        } else {
            time = time.saturating_sub(self.tick_period().mul_f64((last_time_tick.value - self.cur_tick.value) as f64));
        }
        if !self.in_simulation {
            time + self.tick_remainder
        } else {
            time
        }
    }

    pub fn real_time(&self) -> Duration {
        self.real_timer.elapsed()
    }

    pub fn server_time(&self) -> Duration {
        Duration::ZERO
    }

    pub fn frame_time(&self) -> Duration {
        if self.in_simulation {
            self.tick_period()
        } else if self.paused {
            Duration::ZERO
        } else {
            self.real_frame_time()
        }
    }

    pub fn real_frame_time(&self) -> Duration {
        self.duration_from_ticks(self.real_frame_times[self.frame_idx])
    }

    pub fn real_frame_time_avg(&self) -> Duration {
        let sum: i64 = self.real_frame_times.iter().sum();
        self.duration_from_ticks(sum / NUM_FRAMES as i64)
    }

    pub fn real_frame_time_std_dev(&self) -> Duration {
        let sum: i64 = self.real_frame_times.iter().sum();
        let count = self.real_frame_times.len() as f64;
        let avg = sum as f64 / count;
        let mut dev_squared = 0.0_f64;
        for frame_time in self.real_frame_times {
            if frame_time == 0 {
                continue;
            }
            let dt = frame_time as f64 - avg;
            dev_squared += dt * dt;
        }
        let variance = dev_squared / (count - 1.0);
        self.duration_from_ticks(variance.sqrt() as i64)
    }

    pub fn frames_per_second_avg(&self) -> f64 {
        let avg = self.real_frame_times.iter().sum::<i64>() as f64 / NUM_FRAMES as f64;
        if avg == 0.0 {
            0.0
        } else {
            1.0 / (avg / 10_000_000.0)
        }
    }

    pub fn tick_period(&self) -> Duration {
        let ticks = (1.0 / self.tick_rate as f64 * 10_000_000.0) as u64;
        Duration::from_nanos(ticks * 100)
    }

    pub fn tick_fraction(&self) -> u16 {
        if self.in_simulation {
            u16::MAX
        } else {
            (u16::MAX as f64 * self.tick_remainder.as_secs_f64() / self.tick_period().as_secs_f64()) as u16
        }
    }

    pub fn tick_stamp(&self) -> String {
        format!(
            "{}, predFirst: {}, tickRem: {}, sim: {}",
            self.cur_tick,
            self.is_first_time_predicted,
            self.tick_remainder.as_secs_f64(),
            self.in_simulation
        )
    }

    pub fn set_tick_rate(&mut self, tick_rate: u8) {
        if self.tick_rate != 0 {
            self.cache_cur_time();
        }
        self.tick_rate = tick_rate;
    }

    pub fn start_frame(&mut self) {
        let cur_real_time = self.real_time();
        let real_frame_time = cur_real_time.saturating_sub(self.last_real_time);
        self.last_real_time = cur_real_time;
        self.frame_idx = (self.frame_idx + 1) % self.real_frame_times.len();
        self.real_frame_times[self.frame_idx] = real_frame_time.as_nanos() as i64 / 100;
    }

    pub fn reset_sim_time(&mut self) {
        self.cached_cur_time = (Duration::ZERO, GameTick::FIRST);
        self.cur_tick = GameTick::FIRST;
        self.tick_remainder = Duration::ZERO;
        self.paused = true;
    }

    pub fn in_prediction(&self) -> bool {
        self.cur_tick > self.last_real_tick
    }

    fn cache_cur_time(&mut self) {
        let (cached_time, last_time_tick) = self.cached_cur_time;
        let new_time = if self.cur_tick.value >= last_time_tick.value {
            cached_time + self.tick_period().mul_f64((self.cur_tick.value - last_time_tick.value) as f64)
        } else {
            cached_time.saturating_sub(self.tick_period().mul_f64((last_time_tick.value - self.cur_tick.value) as f64))
        };
        self.cached_cur_time = (new_time, self.cur_tick);
    }

    fn duration_from_ticks(&self, ticks: i64) -> Duration {
        if ticks <= 0 {
            Duration::ZERO
        } else {
            Duration::from_nanos((ticks as u64) * 100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GameTiming;
    use crate::{GameTick, Stopwatch};
    use std::time::Duration;

    #[derive(Clone)]
    struct FakeStopwatch {
        elapsed: Duration,
    }

    impl Default for FakeStopwatch {
        fn default() -> Self {
            Self {
                elapsed: Duration::ZERO,
            }
        }
    }

    impl Stopwatch for FakeStopwatch {
        fn elapsed(&self) -> Duration {
            self.elapsed
        }

        fn restart(&mut self) {
            self.elapsed = Duration::ZERO;
        }

        fn start(&mut self) {}
    }

    #[test]
    fn tick_period_matches_tick_rate() {
        let timing = GameTiming::new(FakeStopwatch::default());
        assert_eq!(timing.tick_period(), Duration::from_nanos(16_666_600));
    }

    #[test]
    fn cur_time_advances_with_ticks() {
        let mut timing = GameTiming::new(FakeStopwatch::default());
        timing.cur_tick = GameTick::new(4);
        assert_eq!(timing.cur_time(), Duration::from_nanos(49_999_800));
    }
}
