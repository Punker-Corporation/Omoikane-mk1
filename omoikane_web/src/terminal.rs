use crate::metrics::OmoikaneRuntimeMetrics;
use crate::sql::OmoikaneSqlState;
use daikoku::ServerStatusSnapshot;
use omoikane_control::OmoikaneLaunchManifest;
use std::fmt::Write as _;

pub fn render_boot_panel(
    manifest: &OmoikaneLaunchManifest,
    sql: Option<&OmoikaneSqlState>,
    workers: usize,
) -> String {
    let mut out = String::new();
    push_clear(&mut out);
    push_header(&mut out, "OMOIKANE MK1 :: SERVER ASCENSION");
    push_row(
        &mut out,
        "bind",
        &format!("{}:{}", manifest.endpoint.bind_host, manifest.endpoint.port),
    );
    push_row(&mut out, "local", &manifest.endpoint.local_status_url);
    push_row(&mut out, "public", &manifest.endpoint.public_status_url);
    push_row(
        &mut out,
        "overlay",
        manifest
            .overlay
            .as_ref()
            .map(|overlay| overlay.address.to_string())
            .unwrap_or_else(|| "disabled".to_string())
            .as_str(),
    );
    push_row(&mut out, "workers", &workers.to_string());
    push_row(
        &mut out,
        "sqlx",
        sql.map(|sql| sql.redacted_url()).unwrap_or("disabled"),
    );
    push_row(&mut out, "metrics", "/metrics + /grafana/dashboard.json");
    push_footer(&mut out);
    out
}

pub fn render_live_panel(
    manifest: &OmoikaneLaunchManifest,
    snapshot: &ServerStatusSnapshot,
    metrics: &OmoikaneRuntimeMetrics,
    sql: Option<&OmoikaneSqlState>,
) -> String {
    let mut out = String::new();
    push_clear(&mut out);
    push_header(&mut out, "OMOIKANE LIVE RACK :: OBSERVABILITY SIGIL");
    push_row(&mut out, "state", &format!("{:?}", snapshot.state));
    push_row(&mut out, "tick", &snapshot.tick.value.to_string());
    push_row(&mut out, "tick rate", &format!("{} Hz", snapshot.tick_rate));
    push_row(
        &mut out,
        "players",
        &format!("{}/{}", snapshot.players, snapshot.max_players),
    );
    push_row(
        &mut out,
        "uptime",
        &format!("{}s", metrics.uptime_seconds()),
    );
    push_row(
        &mut out,
        "http req",
        &metrics.http_requests_total().to_string(),
    );
    push_row(
        &mut out,
        "sql checks",
        &metrics.sql_checks_total().to_string(),
    );
    push_row(
        &mut out,
        "sql errors",
        &metrics.sql_errors_total().to_string(),
    );
    push_row(
        &mut out,
        "sqlx",
        sql.map(|sql| format!("{} max_conn={}", sql.redacted_url(), sql.max_connections()))
            .unwrap_or_else(|| "disabled".to_string())
            .as_str(),
    );
    push_row(&mut out, "local", &manifest.endpoint.local_status_url);
    push_row(&mut out, "public", &manifest.endpoint.public_status_url);
    push_machine(&mut out);
    push_footer(&mut out);
    out
}

fn push_clear(out: &mut String) {
    out.push_str("\x1b[2J\x1b[H");
}

fn push_header(out: &mut String, title: &str) {
    out.push_str("\x1b[38;5;81m");
    out.push_str(
        "+------------------------------------------------------------------------------+\n",
    );
    out.push_str("| ");
    out.push_str(title);
    for _ in title.len()..75 {
        out.push(' ');
    }
    out.push_str("|\n");
    out.push_str(
        "+------------------------------------------------------------------------------+\n",
    );
    out.push_str("\x1b[0m");
}

fn push_row(out: &mut String, label: &str, value: &str) {
    out.push_str("\x1b[38;5;245m");
    write!(out, "| {:<12}", label).expect("writing terminal row to String cannot fail");
    out.push_str("\x1b[0m");
    out.push(' ');
    out.push_str("\x1b[38;5;159m");
    let clipped = clip(value, 61);
    write!(out, "{:<61}", clipped).expect("writing terminal row to String cannot fail");
    out.push_str("\x1b[0m");
    out.push_str("|\n");
}

fn push_machine(out: &mut String) {
    push_row(
        out,
        "machine",
        &format!(
            "{} {} {} pid={}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::consts::FAMILY,
            std::process::id()
        ),
    );
    push_row(
        out,
        "parallel",
        &format!(
            "{} hardware threads visible",
            std::thread::available_parallelism().map_or(1, usize::from)
        ),
    );
}

fn push_footer(out: &mut String) {
    out.push_str("\x1b[38;5;81m");
    out.push_str(
        "+------------------------------------------------------------------------------+\n",
    );
    out.push_str("\x1b[0m");
}

fn clip(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in value.chars().take(max_chars) {
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::render_live_panel;
    use crate::metrics::OmoikaneRuntimeMetrics;
    use daikoku::{DaikokuServer, ServerOptions};
    use omoikane_control::OmoikaneLaunchConfig;

    #[test]
    fn live_terminal_contains_runtime_and_machine_sections() {
        let mut server = DaikokuServer::new(ServerOptions::default());
        server.start();
        let snapshot = server.status_snapshot();
        let manifest = OmoikaneLaunchConfig::default().build_manifest().unwrap();
        let terminal = render_live_panel(
            &manifest,
            &snapshot,
            &OmoikaneRuntimeMetrics::default(),
            None,
        );

        assert!(terminal.contains("OMOIKANE LIVE RACK"));
        assert!(terminal.contains("machine"));
        assert!(terminal.contains("http req"));
    }
}
