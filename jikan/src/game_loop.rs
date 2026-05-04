use crate::{FrameEventArgs, GameTick, GameTiming, Stopwatch};
use keisan::MathHelper;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepMode {
    None,
    Yield,
    Delay,
}

pub trait GameLoopHooks {
    fn input(&mut self, _frame: FrameEventArgs) {}
    fn tick(&mut self, _frame: FrameEventArgs) {}
    fn update(&mut self, _frame: FrameEventArgs) {}
    fn render(&mut self, _frame: FrameEventArgs) {}
}

pub struct GameLoop<T: Stopwatch> {
    pub timing: GameTiming<T>,
    pub single_step: bool,
    pub running: bool,
    pub max_queued_ticks: u32,
    pub sleep_mode: SleepMode,
}

impl<T: Stopwatch> GameLoop<T> {
    pub fn new(timing: GameTiming<T>) -> Self {
        Self {
            timing,
            single_step: false,
            running: false,
            max_queued_ticks: 5,
            sleep_mode: SleepMode::Yield,
        }
    }

    pub fn run<H: GameLoopHooks>(&mut self, hooks: &mut H) {
        assert!(
            self.timing.tick_rate > 0,
            "TickRate must be greater than 0."
        );
        self.running = true;
        while self.running {
            let max_time = self
                .timing
                .tick_period()
                .mul_f64(self.max_queued_ticks as f64);
            let mut accumulator = self
                .timing
                .real_time()
                .saturating_sub(self.timing.last_tick);
            if accumulator > max_time {
                accumulator = max_time;
                self.timing.last_tick = self.timing.real_time().saturating_sub(max_time);
            }

            self.timing.start_frame();
            let real_frame = FrameEventArgs::new(self.timing.real_frame_time().as_secs_f32());
            hooks.input(real_frame);
            self.timing.in_simulation = true;
            let mut tick_period = self.calc_tick_period();

            while accumulator >= tick_period {
                accumulator = accumulator.saturating_sub(tick_period);
                self.timing.last_tick += tick_period;
                if self.timing.paused {
                    continue;
                }
                let sim_frame = FrameEventArgs::new(self.timing.frame_time().as_secs_f32());
                hooks.tick(sim_frame);
                self.timing.cur_tick = GameTick::new(self.timing.cur_tick.value + 1);
                tick_period = self.calc_tick_period();
                if self.single_step {
                    self.timing.paused = true;
                }
            }

            if !self.timing.paused {
                self.timing.tick_remainder = accumulator;
            }

            self.timing.in_simulation = false;
            hooks.update(real_frame);
            hooks.render(real_frame);

            match self.sleep_mode {
                SleepMode::None => {}
                SleepMode::Yield => thread::yield_now(),
                SleepMode::Delay => thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    fn calc_tick_period(&self) -> Duration {
        let ratio = MathHelper::clamp(self.timing.tick_timing_adjustment, -0.99, 0.99);
        let diff = self.timing.tick_period().mul_f64(ratio as f64);
        self.timing.tick_period().saturating_sub(diff)
    }
}
