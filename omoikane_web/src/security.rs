use actix_web::{HttpRequest, HttpResponse};
use omoikane_control::OmoikaneLaunchManifest;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct OmoikaneSecurityState {
    enabled: bool,
    max_requests: u32,
    window: Duration,
    clients: Mutex<HashMap<String, ClientWindow>>,
    accepted_total: AtomicU64,
    blocked_total: AtomicU64,
    events_total: AtomicU64,
}

#[derive(Debug)]
struct ClientWindow {
    started: Instant,
    count: u32,
}

impl OmoikaneSecurityState {
    pub fn from_manifest(manifest: &OmoikaneLaunchManifest) -> Self {
        Self {
            enabled: manifest.config.anti_ddos_enabled,
            max_requests: manifest.config.anti_ddos_max_requests,
            window: Duration::from_secs(u64::from(manifest.config.anti_ddos_window_seconds)),
            clients: Mutex::new(HashMap::new()),
            accepted_total: AtomicU64::new(0),
            blocked_total: AtomicU64::new(0),
            events_total: AtomicU64::new(0),
        }
    }

    pub fn guard(&self, req: &HttpRequest) -> Option<HttpResponse> {
        self.accepted_total.fetch_add(1, Ordering::Relaxed);
        if !self.enabled {
            return None;
        }

        let client = req
            .peer_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "local".to_string());
        let Ok(mut clients) = self.clients.lock() else {
            self.events_total.fetch_add(1, Ordering::Relaxed);
            return None;
        };
        let now = Instant::now();
        let window = clients.entry(client.clone()).or_insert(ClientWindow {
            started: now,
            count: 0,
        });
        if now.duration_since(window.started) >= self.window {
            window.started = now;
            window.count = 0;
        }
        window.count = window.count.saturating_add(1);
        if window.count > self.max_requests {
            self.blocked_total.fetch_add(1, Ordering::Relaxed);
            self.events_total.fetch_add(1, Ordering::Relaxed);
            return Some(
                HttpResponse::TooManyRequests()
                    .content_type("application/json")
                    .body(format!(
                        r#"{{"error":"omoikane_anti_ddos","client":"{}","retry_after_seconds":{}}}"#,
                        json_escape(&client),
                        self.window.as_secs()
                    )),
            );
        }
        None
    }

    pub fn accepted_total(&self) -> u64 {
        self.accepted_total.load(Ordering::Relaxed)
    }

    pub fn blocked_total(&self) -> u64 {
        self.blocked_total.load(Ordering::Relaxed)
    }

    pub fn events_total(&self) -> u64 {
        self.events_total.load(Ordering::Relaxed)
    }

    pub fn active_clients(&self) -> usize {
        self.clients.lock().map_or(0, |clients| clients.len())
    }

    pub fn write_json(&self, out: &mut String) {
        out.clear();
        out.push('{');
        write!(out, r#""enabled":{}"#, self.enabled).expect("writing security json");
        write!(out, r#","window_seconds":{}"#, self.window.as_secs())
            .expect("writing security json");
        write!(out, r#","max_requests":{}"#, self.max_requests).expect("writing security json");
        write!(out, r#","accepted_total":{}"#, self.accepted_total())
            .expect("writing security json");
        write!(out, r#","blocked_total":{}"#, self.blocked_total()).expect("writing security json");
        write!(out, r#","events_total":{}"#, self.events_total()).expect("writing security json");
        write!(out, r#","active_clients":{}"#, self.active_clients())
            .expect("writing security json");
        out.push_str(
            r#","posture":"rate-limit, invalid-drop, sql-health, dns-watch, rack-firewall""#,
        );
        out.push('}');
    }

    pub fn write_prometheus(&self, out: &mut String) {
        out.push_str("# HELP omoikane_security_requests_total Requests inspected by Omoikane security guard.\n");
        out.push_str("# TYPE omoikane_security_requests_total counter\n");
        writeln!(
            out,
            "omoikane_security_requests_total {}",
            self.accepted_total()
        )
        .expect("writing metrics to String cannot fail");
        out.push_str(
            "# HELP omoikane_security_blocks_total Requests blocked by Omoikane security guard.\n",
        );
        out.push_str("# TYPE omoikane_security_blocks_total counter\n");
        writeln!(
            out,
            "omoikane_security_blocks_total {}",
            self.blocked_total()
        )
        .expect("writing metrics to String cannot fail");
        out.push_str(
            "# HELP omoikane_security_active_clients Active client buckets in the rate window.\n",
        );
        out.push_str("# TYPE omoikane_security_active_clients gauge\n");
        writeln!(
            out,
            "omoikane_security_active_clients {}",
            self.active_clients()
        )
        .expect("writing metrics to String cannot fail");
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::OmoikaneSecurityState;
    use actix_web::{http::StatusCode, test};
    use omoikane_control::OmoikaneLaunchConfig;

    #[test]
    fn security_guard_blocks_after_window_budget() {
        let manifest = OmoikaneLaunchConfig {
            anti_ddos_max_requests: 1,
            ..OmoikaneLaunchConfig::default()
        }
        .build_manifest()
        .unwrap();
        let security = OmoikaneSecurityState::from_manifest(&manifest);
        let req = test::TestRequest::get().uri("/status").to_http_request();

        assert!(security.guard(&req).is_none());
        let blocked = security.guard(&req).expect("second request is blocked");
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
