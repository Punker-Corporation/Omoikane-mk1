use daikoku::ServerStatusSnapshot;
use omoikane_control::OmoikaneLaunchManifest;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug)]
pub struct OmoikaneRuntimeMetrics {
    started: Instant,
    http_requests_total: AtomicU64,
    sql_checks_total: AtomicU64,
    sql_errors_total: AtomicU64,
}

impl Default for OmoikaneRuntimeMetrics {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            http_requests_total: AtomicU64::new(0),
            sql_checks_total: AtomicU64::new(0),
            sql_errors_total: AtomicU64::new(0),
        }
    }
}

impl OmoikaneRuntimeMetrics {
    pub fn record_http_request(&self) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_sql_check(&self) {
        self.sql_checks_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_sql_error(&self) {
        self.sql_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    pub fn http_requests_total(&self) -> u64 {
        self.http_requests_total.load(Ordering::Relaxed)
    }

    pub fn sql_checks_total(&self) -> u64 {
        self.sql_checks_total.load(Ordering::Relaxed)
    }

    pub fn sql_errors_total(&self) -> u64 {
        self.sql_errors_total.load(Ordering::Relaxed)
    }

    pub fn write_prometheus(
        &self,
        snapshot: &ServerStatusSnapshot,
        manifest: &OmoikaneLaunchManifest,
        sql_enabled: bool,
        out: &mut String,
    ) {
        out.clear();
        out.push_str("# HELP omoikane_uptime_seconds Process uptime.\n");
        out.push_str("# TYPE omoikane_uptime_seconds gauge\n");
        writeln!(out, "omoikane_uptime_seconds {}", self.uptime_seconds())
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_http_requests_total HTTP requests handled by Omoikane.\n");
        out.push_str("# TYPE omoikane_http_requests_total counter\n");
        writeln!(
            out,
            "omoikane_http_requests_total {}",
            self.http_requests_total()
        )
        .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_tick Current authoritative simulation tick.\n");
        out.push_str("# TYPE omoikane_tick gauge\n");
        writeln!(out, "omoikane_tick {}", snapshot.tick.value)
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_players Connected players.\n");
        out.push_str("# TYPE omoikane_players gauge\n");
        writeln!(out, "omoikane_players {}", snapshot.players)
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_max_players Configured player capacity.\n");
        out.push_str("# TYPE omoikane_max_players gauge\n");
        writeln!(out, "omoikane_max_players {}", snapshot.max_players)
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_tick_rate Configured server tick rate.\n");
        out.push_str("# TYPE omoikane_tick_rate gauge\n");
        writeln!(out, "omoikane_tick_rate {}", snapshot.tick_rate)
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_sql_enabled SQLx pool enabled for this process.\n");
        out.push_str("# TYPE omoikane_sql_enabled gauge\n");
        writeln!(out, "omoikane_sql_enabled {}", u8::from(sql_enabled))
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_sql_checks_total SQL liveness checks attempted.\n");
        out.push_str("# TYPE omoikane_sql_checks_total counter\n");
        writeln!(out, "omoikane_sql_checks_total {}", self.sql_checks_total())
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_sql_errors_total SQL liveness checks that failed.\n");
        out.push_str("# TYPE omoikane_sql_errors_total counter\n");
        writeln!(out, "omoikane_sql_errors_total {}", self.sql_errors_total())
            .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_machine_parallelism Available CPU parallelism.\n");
        out.push_str("# TYPE omoikane_machine_parallelism gauge\n");
        writeln!(
            out,
            "omoikane_machine_parallelism {}",
            std::thread::available_parallelism().map_or(1, usize::from)
        )
        .expect("writing metrics to String cannot fail");
        out.push_str("# HELP omoikane_overlay_enabled Overlay profile enabled.\n");
        out.push_str("# TYPE omoikane_overlay_enabled gauge\n");
        writeln!(
            out,
            "omoikane_overlay_enabled {}",
            u8::from(manifest.overlay.is_some())
        )
        .expect("writing metrics to String cannot fail");
    }

    pub fn write_grakane_dashboard(&self, out: &mut String) {
        out.clear();
        out.push_str(
            r#"{"title":"Omoikane Grakane Runtime","timezone":"browser","schemaVersion":39,"version":1,"refresh":"1s","tags":["omoikane","grakane","functional-stage"],"panels":["#,
        );
        push_stat_panel(out, 1, "Uptime", "omoikane_uptime_seconds", 0, 0);
        out.push(',');
        push_stat_panel(
            out,
            2,
            "HTTP Requests",
            "omoikane_http_requests_total",
            6,
            0,
        );
        out.push(',');
        push_stat_panel(out, 3, "Tick", "omoikane_tick", 12, 0);
        out.push(',');
        push_stat_panel(out, 4, "Players", "omoikane_players", 18, 0);
        out.push(',');
        push_stat_panel(out, 5, "SQL Errors", "omoikane_sql_errors_total", 0, 8);
        out.push(',');
        push_stat_panel(
            out,
            6,
            "Machine Parallelism",
            "omoikane_machine_parallelism",
            6,
            8,
        );
        out.push_str("]}");
    }
}

fn push_stat_panel(out: &mut String, id: u16, title: &str, expr: &str, x: u16, y: u16) {
    write!(
        out,
        r#"{{"id":{id},"type":"stat","title":"{title}","gridPos":{{"x":{x},"y":{y},"w":6,"h":8}},"targets":[{{"expr":"{expr}","refId":"A"}}]}}"#
    )
    .expect("writing dashboard to String cannot fail");
}

#[cfg(test)]
mod tests {
    use super::OmoikaneRuntimeMetrics;
    use daikoku::{DaikokuServer, ServerOptions};
    use omoikane_control::OmoikaneLaunchConfig;

    #[test]
    fn metrics_emit_prometheus_and_grakane_shapes() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        server.tick_update(0.016);
        let snapshot = server.status_snapshot();
        let manifest = OmoikaneLaunchConfig::default().build_manifest().unwrap();
        let metrics = OmoikaneRuntimeMetrics::default();
        metrics.record_http_request();

        let mut out = String::new();
        metrics.write_prometheus(&snapshot, &manifest, false, &mut out);
        assert!(out.contains("omoikane_http_requests_total 1"));
        assert!(out.contains("omoikane_tick 1"));

        metrics.write_grakane_dashboard(&mut out);
        assert!(out.contains("\"title\":\"Omoikane Grakane Runtime\""));
        assert!(out.contains("omoikane_machine_parallelism"));
    }
}
