use crate::security::OmoikaneSecurityState;
use omoikane_control::OmoikaneLaunchManifest;
use std::fmt::Write as _;

pub fn write_sentinel_json(
    manifest: &OmoikaneLaunchManifest,
    security: &OmoikaneSecurityState,
    out: &mut String,
) {
    out.clear();
    let dns_risk = phishing_risk_score(&manifest.dns.selected_host);
    out.push('{');
    write!(out, r#""name":"Omoikane Kanshi Sentinel""#).expect("writing sentinel json");
    write!(
        out,
        r#","dns_host":"{}""#,
        json_escape(&manifest.dns.selected_host)
    )
    .expect("writing sentinel json");
    write!(out, r#","phishing_risk_score":{dns_risk}"#).expect("writing sentinel json");
    write!(
        out,
        r#","phishing_signals":["{}"]"#,
        json_escape(&phishing_signal(&manifest.dns.selected_host))
    )
    .expect("writing sentinel json");
    write!(
        out,
        r#","forensics":{{"os":"{}","arch":"{}","family":"{}","pid":{},"parallelism":{}}}"#,
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY,
        std::process::id(),
        std::thread::available_parallelism().map_or(1, usize::from)
    )
    .expect("writing sentinel json");
    write!(
        out,
        r#","security":{{"blocked_total":{},"events_total":{},"active_clients":{}}}"#,
        security.blocked_total(),
        security.events_total(),
        security.active_clients()
    )
    .expect("writing sentinel json");
    out.push_str(
        r#","rack_interfaces":["michisuji-routeros-terminal","kaminari-juniper-terminal"]"#,
    );
    out.push_str(r#","mode":"rust-native-phishing-watch-and-endpoint-forensics""#);
    out.push('}');
}

fn phishing_risk_score(host: &str) -> u8 {
    let lower = host.to_ascii_lowercase();
    let mut score = 0u8;
    for marker in [
        "login", "secure", "verify", "account", "wallet", "paypal", "bank", "gmail",
    ] {
        if lower.contains(marker) {
            score = score.saturating_add(12);
        }
    }
    if lower.starts_with("xn--") || lower.contains(".xn--") {
        score = score.saturating_add(30);
    }
    if host.len() > 48 {
        score = score.saturating_add(16);
    }
    score.min(100)
}

fn phishing_signal(host: &str) -> String {
    let score = phishing_risk_score(host);
    if score == 0 {
        "dns limpo para o perfil atual".to_string()
    } else if score < 35 {
        "marcadores fracos no DNS escolhido".to_string()
    } else {
        "DNS escolhido exige revisao manual antes de producao".to_string()
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
    use super::write_sentinel_json;
    use crate::security::OmoikaneSecurityState;
    use omoikane_control::OmoikaneLaunchConfig;

    #[test]
    fn sentinel_json_reports_host_forensics_and_dns_risk() {
        let manifest = OmoikaneLaunchConfig {
            public_dns_name: Some("secure-login-rack.example".to_string()),
            ..OmoikaneLaunchConfig::default()
        }
        .build_manifest()
        .unwrap();
        let security = OmoikaneSecurityState::from_manifest(&manifest);
        let mut out = String::new();

        write_sentinel_json(&manifest, &security, &mut out);

        assert!(out.contains("\"phishing_risk_score\":"));
        assert!(out.contains("\"forensics\""));
        assert!(out.contains("michisuji-routeros-terminal"));
    }
}
