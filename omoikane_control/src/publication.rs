use crate::json;
use std::fmt::Write as _;
use std::net::{IpAddr, Ipv4Addr};

pub const DEFAULT_GAME_SERVER_PORT: u16 = 7777;
pub const DEFAULT_VPS_REALITY_PORT: u16 = 443;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikanePublicationPlan {
    pub base_host: String,
    pub target_host: String,
    pub http_port: u16,
    pub public_site_enabled: bool,
    pub dns_records: Vec<OmoikaneDnsRecord>,
    pub subservers: Vec<OmoikaneSubserver>,
    pub vps: OmoikaneVpsBlueprint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikaneDnsRecord {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl_seconds: u16,
    pub service_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikaneSubserver {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub public_url: String,
    pub enabled: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikaneVpsBlueprint {
    pub enabled: bool,
    pub entry_host: String,
    pub entry_port: u16,
    pub reality_sni: Option<String>,
    pub reference_model: String,
    pub operator_inputs: Vec<String>,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikanePublicationOptions {
    pub base_host: String,
    pub target_host: String,
    pub http_port: u16,
    pub public_site_enabled: bool,
    pub game_server_enabled: bool,
    pub game_server_port: u16,
    pub vps_mode_enabled: bool,
    pub vps_reality_sni: Option<String>,
}

impl OmoikanePublicationPlan {
    pub fn new(options: OmoikanePublicationOptions) -> Self {
        let base_host = options.base_host.trim().to_string();
        let target_host = options.target_host.trim().to_string();
        let http_port = options.http_port;
        let can_publish_dns = is_publishable_dns_name(&base_host);

        let mut subservers = Vec::new();
        push_subserver(
            &mut subservers,
            OmoikaneSubserver::http(HttpSubserverSpec {
                id: "site",
                label: "Site publico de teste",
                kind: "public-site",
                host: host_for_service(&base_host, "site", can_publish_dns, false),
                port: http_port,
                path: "/",
                enabled: options.public_site_enabled,
                status: "active",
            }),
        );
        push_subserver(
            &mut subservers,
            OmoikaneSubserver::http(HttpSubserverSpec {
                id: "console",
                label: "Console Mikado",
                kind: "ops-console",
                host: host_for_service(&base_host, "console", can_publish_dns, true),
                port: http_port,
                path: "/console",
                enabled: true,
                status: "active",
            }),
        );
        push_subserver(
            &mut subservers,
            OmoikaneSubserver::http(HttpSubserverSpec {
                id: "status",
                label: "Status autoritativo",
                kind: "observability",
                host: host_for_service(&base_host, "status", can_publish_dns, true),
                port: http_port,
                path: "/status",
                enabled: true,
                status: "active",
            }),
        );
        push_subserver(
            &mut subservers,
            OmoikaneSubserver::http(HttpSubserverSpec {
                id: "metrics",
                label: "Metricas Prometheus-style",
                kind: "observability",
                host: host_for_service(&base_host, "metrics", can_publish_dns, true),
                port: http_port,
                path: "/metrics",
                enabled: true,
                status: "active",
            }),
        );
        push_subserver(
            &mut subservers,
            OmoikaneSubserver::udp(
                "game",
                "Servidor de jogo autoritativo",
                host_for_service(&base_host, "game", can_publish_dns, true),
                options.game_server_port,
                options.game_server_enabled,
                if options.game_server_enabled {
                    "reserved"
                } else {
                    "planned"
                },
            ),
        );

        let vps_host = host_for_service(&base_host, "vps", can_publish_dns, true);
        let vps = OmoikaneVpsBlueprint::new(
            options.vps_mode_enabled,
            vps_host.clone(),
            DEFAULT_VPS_REALITY_PORT,
            options.vps_reality_sni,
        );
        push_subserver(
            &mut subservers,
            OmoikaneSubserver::tcp_tls(
                "vps-reality",
                "Entrada VPS VLESS Reality",
                "vps-edge",
                vps_host,
                DEFAULT_VPS_REALITY_PORT,
                options.vps_mode_enabled,
                if options.vps_mode_enabled {
                    "operator-managed"
                } else {
                    "blueprint"
                },
            ),
        );

        let dns_records = dns_records_for(&base_host, &target_host, &subservers, can_publish_dns);

        Self {
            base_host,
            target_host,
            http_port,
            public_site_enabled: options.public_site_enabled,
            dns_records,
            subservers,
            vps,
        }
    }

    pub fn public_site_url(&self) -> String {
        self.subservers
            .iter()
            .find(|server| server.id == "site")
            .map(|server| server.public_url.clone())
            .unwrap_or_else(|| http_url(&self.base_host, self.http_port, "/"))
    }

    pub fn render_routeros_dns_script(&self) -> String {
        let mut script = String::new();
        script.push_str("# Omoikane public DNS publication plan for RouterOS\n");
        script.push_str("# This configures local RouterOS DNS hints only. Registrar or authoritative DNS still needs real credentials.\n");
        script.push_str("# Review every record before applying in production.\n");
        script.push_str("/ip dns set allow-remote-requests=yes\n");
        script.push_str("/ip dns static remove [find comment~\"omoikane publication\"]\n");
        if self.dns_records.is_empty() {
            script.push_str("# No public DNS records were generated because the selected host is not a publishable DNS name.\n");
            return script;
        }
        for record in &self.dns_records {
            let name = routeros_quote(&record.name);
            let value = routeros_quote(&record.value);
            let ttl = record.ttl_seconds;
            let comment = routeros_quote(&format!("omoikane publication {}", record.service_id));
            match record.record_type.as_str() {
                "CNAME" => {
                    writeln!(
                        script,
                        "/ip dns static add name={name} type=CNAME cname={value} ttl={ttl}s comment={comment}"
                    )
                    .expect("writing RouterOS script to String cannot fail");
                }
                "AAAA" => {
                    writeln!(
                        script,
                        "/ip dns static add name={name} type=AAAA address={value} ttl={ttl}s comment={comment}"
                    )
                    .expect("writing RouterOS script to String cannot fail");
                }
                _ => {
                    writeln!(
                        script,
                        "/ip dns static add name={name} address={value} ttl={ttl}s comment={comment}"
                    )
                    .expect("writing RouterOS script to String cannot fail");
                }
            }
        }
        script
    }

    pub fn render_junos_dns_set(&self) -> String {
        let mut script = String::new();
        script.push_str("# Omoikane public DNS publication plan for Junos\n");
        script
            .push_str("# Static host mappings are local resolver hints, not registrar updates.\n");
        script.push_str("# CNAME records are listed as comments because Junos static-host-mapping stores address records.\n");
        if self.dns_records.is_empty() {
            script.push_str("# No public DNS records were generated because the selected host is not a publishable DNS name.\n");
            return script;
        }
        for record in &self.dns_records {
            match record.record_type.as_str() {
                "A" => {
                    writeln!(
                        script,
                        "set system static-host-mapping host-name {} inet {}",
                        junos_token(&record.name),
                        junos_token(&record.value)
                    )
                    .expect("writing Junos set commands to String cannot fail");
                }
                "AAAA" => {
                    writeln!(
                        script,
                        "set system static-host-mapping host-name {} inet6 {}",
                        junos_token(&record.name),
                        junos_token(&record.value)
                    )
                    .expect("writing Junos set commands to String cannot fail");
                }
                "CNAME" => {
                    writeln!(
                        script,
                        "# CNAME {} -> {} for {}",
                        record.name, record.value, record.service_id
                    )
                    .expect("writing Junos set commands to String cannot fail");
                }
                _ => {}
            }
        }
        script
    }

    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "base_host", &self.base_host, true);
        json::push_string_field(out, "target_host", &self.target_host, false);
        json::push_u16_field(out, "http_port", self.http_port, false);
        json::push_bool_field(out, "public_site_enabled", self.public_site_enabled, false);
        json::push_string_field(out, "public_site_url", &self.public_site_url(), false);
        json::push_string_field(
            out,
            "routeros_script_url",
            "/network/dns/routeros.rsc",
            false,
        );
        json::push_string_field(out, "junos_set_url", "/network/dns/junos.set", false);
        json::push_field_name(out, "dns_records", false);
        out.push('[');
        for (index, record) in self.dns_records.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            record.write_json(out);
        }
        out.push(']');
        json::push_field_name(out, "subservers", false);
        out.push('[');
        for (index, server) in self.subservers.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            server.write_json(out);
        }
        out.push(']');
        json::push_field_name(out, "vps", false);
        self.vps.write_json(out);
        out.push('}');
    }

    pub fn write_subservers_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "base_host", &self.base_host, true);
        json::push_string_field(out, "target_host", &self.target_host, false);
        json::push_field_name(out, "subservers", false);
        out.push('[');
        for (index, server) in self.subservers.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            server.write_json(out);
        }
        out.push(']');
        out.push('}');
    }

    pub fn write_vps_json(&self, out: &mut String) {
        self.vps.write_json(out);
    }
}

impl OmoikaneDnsRecord {
    fn new(
        service_id: impl Into<String>,
        name: impl Into<String>,
        record_type: impl Into<String>,
        value: impl Into<String>,
        ttl_seconds: u16,
    ) -> Self {
        Self {
            service_id: service_id.into(),
            name: name.into(),
            record_type: record_type.into(),
            value: value.into(),
            ttl_seconds,
        }
    }

    fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "service_id", &self.service_id, true);
        json::push_string_field(out, "name", &self.name, false);
        json::push_string_field(out, "record_type", &self.record_type, false);
        json::push_string_field(out, "value", &self.value, false);
        json::push_u16_field(out, "ttl_seconds", self.ttl_seconds, false);
        out.push('}');
    }
}

impl OmoikaneSubserver {
    fn http(spec: HttpSubserverSpec<'_>) -> Self {
        Self {
            id: spec.id.to_string(),
            label: spec.label.to_string(),
            kind: spec.kind.to_string(),
            protocol: "http".to_string(),
            public_url: http_url(&spec.host, spec.port, spec.path),
            host: spec.host,
            port: spec.port,
            path: spec.path.to_string(),
            enabled: spec.enabled,
            status: spec.status.to_string(),
        }
    }

    fn udp(id: &str, label: &str, host: String, port: u16, enabled: bool, status: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            kind: "game-transport".to_string(),
            protocol: "udp".to_string(),
            public_url: format!("udp://{host}:{port}"),
            host,
            port,
            path: String::new(),
            enabled,
            status: status.to_string(),
        }
    }

    fn tcp_tls(
        id: &str,
        label: &str,
        kind: &str,
        host: String,
        port: u16,
        enabled: bool,
        status: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            protocol: "tcp+tls".to_string(),
            public_url: format!("tcp+tls://{host}:{port}"),
            host,
            port,
            path: String::new(),
            enabled,
            status: status.to_string(),
        }
    }

    fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "id", &self.id, true);
        json::push_string_field(out, "label", &self.label, false);
        json::push_string_field(out, "kind", &self.kind, false);
        json::push_string_field(out, "protocol", &self.protocol, false);
        json::push_string_field(out, "host", &self.host, false);
        json::push_u16_field(out, "port", self.port, false);
        json::push_string_field(out, "path", &self.path, false);
        json::push_string_field(out, "public_url", &self.public_url, false);
        json::push_bool_field(out, "enabled", self.enabled, false);
        json::push_string_field(out, "status", &self.status, false);
        out.push('}');
    }
}

struct HttpSubserverSpec<'a> {
    id: &'a str,
    label: &'a str,
    kind: &'a str,
    host: String,
    port: u16,
    path: &'a str,
    enabled: bool,
    status: &'a str,
}

impl OmoikaneVpsBlueprint {
    fn new(
        enabled: bool,
        entry_host: String,
        entry_port: u16,
        reality_sni: Option<String>,
    ) -> Self {
        Self {
            enabled,
            entry_host,
            entry_port,
            reality_sni,
            reference_model: "Xray VLESS Reality operator-managed edge".to_string(),
            operator_inputs: vec![
                "VPS with a public IPv4 or IPv6 address".to_string(),
                "registered DNS record pointing at the VPS".to_string(),
                "operator-generated Xray keys, UUIDs and short IDs".to_string(),
                "firewall rule for the chosen TLS/Reality entry port".to_string(),
            ],
            safety_notes: vec![
                "Omoikane does not vendor Xray, 3x-ui, firmware or third-party install scripts"
                    .to_string(),
                "Reality/VLESS credentials must be generated and rotated by the operator"
                    .to_string(),
                "Use this as a publication blueprint, not as an automatic proxy installer"
                    .to_string(),
            ],
        }
    }

    fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_bool_field(out, "enabled", self.enabled, true);
        json::push_string_field(out, "entry_host", &self.entry_host, false);
        json::push_u16_field(out, "entry_port", self.entry_port, false);
        json::push_field_name(out, "reality_sni", false);
        if let Some(sni) = &self.reality_sni {
            json::push_string(out, sni);
        } else {
            out.push_str("null");
        }
        json::push_string_field(out, "reference_model", &self.reference_model, false);
        json::push_field_name(out, "operator_inputs", false);
        write_string_array(out, &self.operator_inputs);
        json::push_field_name(out, "safety_notes", false);
        write_string_array(out, &self.safety_notes);
        out.push('}');
    }
}

fn dns_records_for(
    base_host: &str,
    target_host: &str,
    subservers: &[OmoikaneSubserver],
    can_publish_dns: bool,
) -> Vec<OmoikaneDnsRecord> {
    if !can_publish_dns {
        return Vec::new();
    }

    let mut records = Vec::new();
    let apex_type = dns_record_type_for_target(target_host);
    records.push(OmoikaneDnsRecord::new(
        "site",
        base_host,
        apex_type,
        target_host,
        30,
    ));
    for server in subservers {
        if server.host.eq_ignore_ascii_case(base_host) {
            continue;
        }
        push_unique_record(
            &mut records,
            OmoikaneDnsRecord::new(&server.id, &server.host, "CNAME", base_host, 30),
        );
    }
    records
}

fn dns_record_type_for_target(target: &str) -> &'static str {
    match target.parse::<IpAddr>() {
        Ok(IpAddr::V4(_)) => "A",
        Ok(IpAddr::V6(_)) => "AAAA",
        Err(_) => "CNAME",
    }
}

fn host_for_service(
    base_host: &str,
    service: &str,
    can_publish_dns: bool,
    prefer_subdomain: bool,
) -> String {
    if prefer_subdomain && can_publish_dns {
        format!("{service}.{base_host}")
    } else {
        base_host.to_string()
    }
}

fn http_url(host: &str, port: u16, path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    if port == 80 {
        format!("http://{host}{path}")
    } else {
        format!("http://{host}:{port}{path}")
    }
}

fn push_subserver(subservers: &mut Vec<OmoikaneSubserver>, server: OmoikaneSubserver) {
    if subservers.iter().any(|existing| existing.id == server.id) {
        return;
    }
    subservers.push(server);
}

fn push_unique_record(records: &mut Vec<OmoikaneDnsRecord>, record: OmoikaneDnsRecord) {
    if records
        .iter()
        .any(|existing| existing.name.eq_ignore_ascii_case(&record.name))
    {
        return;
    }
    records.push(record);
}

fn is_publishable_dns_name(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.');
    if host.is_empty()
        || host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.parse::<IpAddr>().is_ok()
    {
        return false;
    }
    let labels = host.split('.').collect::<Vec<_>>();
    if labels.len() < 2 {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn write_string_array(out: &mut String, values: &[String]) {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        json::push_string(out, value);
    }
    out.push(']');
}

fn routeros_quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '"' | '\\' | '$' | '`' => {
                quoted.push('\\');
                quoted.push(ch);
            }
            '\n' | '\r' => quoted.push(' '),
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

fn junos_token(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'-' | b'_'))
    {
        return value.to_string();
    }

    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '"' | '\\' => {
                quoted.push('\\');
                quoted.push(ch);
            }
            '\n' | '\r' => quoted.push(' '),
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

pub fn is_private_or_local_target(target: &str) -> bool {
    let Ok(ip) = target.parse::<IpAddr>() else {
        return false;
    };
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip == Ipv4Addr::new(0, 0, 0, 0)
                || ip.octets()[0] == 100
        }
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_GAME_SERVER_PORT, OmoikanePublicationOptions, OmoikanePublicationPlan,
        is_private_or_local_target,
    };

    #[test]
    fn publication_plan_generates_subservers_and_dns_records() {
        let plan = OmoikanePublicationPlan::new(OmoikanePublicationOptions {
            base_host: "omoikane.example".to_string(),
            target_host: "203.0.113.10".to_string(),
            http_port: 8080,
            public_site_enabled: true,
            game_server_enabled: true,
            game_server_port: DEFAULT_GAME_SERVER_PORT,
            vps_mode_enabled: true,
            vps_reality_sni: Some("example-cdn.test".to_string()),
        });

        assert_eq!(plan.public_site_url(), "http://omoikane.example:8080/");
        assert!(
            plan.subservers
                .iter()
                .any(|server| server.host == "console.omoikane.example")
        );
        assert!(
            plan.dns_records
                .iter()
                .any(|record| record.name == "omoikane.example" && record.record_type == "A")
        );
        assert!(
            plan.render_routeros_dns_script()
                .contains("/ip dns static add")
        );
        assert!(
            plan.render_junos_dns_set()
                .contains("set system static-host-mapping")
        );
    }

    #[test]
    fn publication_plan_avoids_fake_dns_for_local_hosts() {
        let plan = OmoikanePublicationPlan::new(OmoikanePublicationOptions {
            base_host: "127.0.0.1".to_string(),
            target_host: "127.0.0.1".to_string(),
            http_port: 8080,
            public_site_enabled: true,
            game_server_enabled: false,
            game_server_port: DEFAULT_GAME_SERVER_PORT,
            vps_mode_enabled: false,
            vps_reality_sni: None,
        });

        assert!(plan.dns_records.is_empty());
        assert!(
            plan.render_routeros_dns_script()
                .contains("not a publishable DNS name")
        );
        assert!(is_private_or_local_target("100.104.1.2"));
    }
}
