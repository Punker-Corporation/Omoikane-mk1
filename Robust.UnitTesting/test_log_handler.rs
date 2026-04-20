use std::io::{self, Write};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use robust_shared::configuration::ConfigurationManager;
use robust_shared::log::{LogEvent, LogHandler, LogLevel};

pub struct TestLogHandler {
    prefix: Option<String>,
    writer: Box<dyn Write + Send>,
    start: Instant,
    failure_level: Arc<RwLock<Option<LogLevel>>>,
}

impl TestLogHandler {
    pub fn new(cfg: &ConfigurationManager, prefix: Option<String>) -> Self {
        let failure_level = Arc::new(RwLock::new(None));
        let failure_level_clone = failure_level.clone();
        cfg.on_value_changed(
            "failure_log_level",
            move |value: Option<LogLevel>| {
                *failure_level_clone.write().unwrap() = value;
            },
            true,
        );

        let mut writer = io::stdout();
        let prefix_str = prefix.clone().unwrap_or_default();
        writeln!(
            writer,
            "{}Started {}",
            if prefix_str.is_empty() { "" } else { &format!("{}: ", prefix_str) },
            chrono::Local::now().to_rfc3339()
        )
        .unwrap();

        Self {
            prefix,
            writer: Box::new(writer),
            start: Instant::now(),
            failure_level,
        }
    }

    fn get_prefix(&self) -> String {
        self.prefix
            .as_ref()
            .map(|p| format!("{}: ", p))
            .unwrap_or_default()
    }
}

impl LogHandler for TestLogHandler {
    fn log(&mut self, sawmill_name: &str, event: &LogEvent) {
        let level = event.level;
        let level_name = level.as_str();
        let elapsed = self.start.elapsed().as_secs_f64();
        let rendered = event.message();
        let line = format!(
            "{}{:.3}s [{}] {}: {}",
            self.get_prefix(),
            elapsed,
            level_name,
            sawmill_name,
            rendered
        );

        writeln!(self.writer, "{}", line).unwrap();

        let failure_guard = self.failure_level.read().unwrap();
        if let Some(failure_lvl) = *failure_guard {
            if level >= failure_lvl {
                drop(failure_guard);
                self.writer.flush().unwrap();
                panic!(
                    "{} Exception: {:?}",
                    line,
                    event.exception().unwrap_or(&"<no exception>".to_string())
                );
            }
        }
    }
}
