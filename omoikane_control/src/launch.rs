use crate::dns::OmoikaneDnsPlan;
use crate::json;
use crate::kaminari::{KaminariDeviceProfile, KaminariMcpCatalog};
use crate::mamori::MamoriPlan;
use crate::overlay::OverlayFixedIpProfile;
use crate::publication::{
    DEFAULT_GAME_SERVER_PORT, OmoikanePublicationOptions, OmoikanePublicationPlan,
};
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikaneLaunchConfig {
    pub server_name: String,
    pub bind_host: String,
    pub port: u16,
    pub max_players: usize,
    pub tick_rate: u16,
    pub overlay_seed: String,
    pub overlay_enabled: bool,
    pub overlay_endpoint_hint: Option<String>,
    pub database_url: Option<String>,
    pub database_max_connections: u32,
    pub kaminari_host: String,
    pub kaminari_username: String,
    pub public_dns_name: Option<String>,
    pub public_dns_target: Option<String>,
    pub grakane_admin_gmail: Option<String>,
    pub anti_ddos_enabled: bool,
    pub anti_ddos_window_seconds: u16,
    pub anti_ddos_max_requests: u32,
    pub public_site_enabled: bool,
    pub game_server_enabled: bool,
    pub game_server_port: u16,
    pub vps_mode_enabled: bool,
    pub vps_reality_sni: Option<String>,
}

impl Default for OmoikaneLaunchConfig {
    fn default() -> Self {
        Self {
            server_name: "Omoikane".to_string(),
            bind_host: "0.0.0.0".to_string(),
            port: 8080,
            max_players: 128,
            tick_rate: 144,
            overlay_seed: "omoikane-rack".to_string(),
            overlay_enabled: true,
            overlay_endpoint_hint: None,
            database_url: None,
            database_max_connections: 16,
            kaminari_host: "192.168.88.1".to_string(),
            kaminari_username: "netops".to_string(),
            public_dns_name: None,
            public_dns_target: None,
            grakane_admin_gmail: None,
            anti_ddos_enabled: true,
            anti_ddos_window_seconds: 60,
            anti_ddos_max_requests: 900,
            public_site_enabled: true,
            game_server_enabled: false,
            game_server_port: DEFAULT_GAME_SERVER_PORT,
            vps_mode_enabled: false,
            vps_reality_sni: None,
        }
    }
}

impl OmoikaneLaunchConfig {
    pub fn build_manifest(&self) -> Result<OmoikaneLaunchManifest, LaunchConfigError> {
        self.validate()?;

        let overlay = if self.overlay_enabled {
            let mut overlay =
                OverlayFixedIpProfile::omoikane(&self.server_name, self.port, &self.overlay_seed);
            if let Some(endpoint_hint) = &self.overlay_endpoint_hint {
                overlay = overlay.with_endpoint_hint(endpoint_hint.clone());
            }
            Some(overlay)
        } else {
            None
        };

        let public_address = overlay
            .as_ref()
            .map(|overlay| overlay.address.to_string())
            .unwrap_or_else(|| self.bind_host.clone());
        let dns = OmoikaneDnsPlan::new(
            &self.bind_host,
            &public_address,
            self.port,
            self.public_dns_name.as_deref(),
        );
        let kaminari_device = KaminariDeviceProfile::rack_default(
            "rack-core",
            &self.kaminari_host,
            &self.kaminari_username,
        );
        let publication_target = self
            .public_dns_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                self.overlay_endpoint_hint
                    .as_deref()
                    .and_then(endpoint_host_hint)
            })
            .unwrap_or_else(|| public_address.clone());
        let publication = OmoikanePublicationPlan::new(OmoikanePublicationOptions {
            base_host: dns.selected_host.clone(),
            target_host: publication_target,
            http_port: self.port,
            public_site_enabled: self.public_site_enabled,
            game_server_enabled: self.game_server_enabled,
            game_server_port: self.game_server_port,
            vps_mode_enabled: self.vps_mode_enabled,
            vps_reality_sni: self.vps_reality_sni.clone(),
        });

        Ok(OmoikaneLaunchManifest {
            endpoint: ServerEndpoint {
                bind_host: self.bind_host.clone(),
                port: self.port,
                local_status_url: self.local_status_url(),
                public_status_url: dns.access_url("/status"),
            },
            config: self.clone(),
            dns,
            publication,
            overlay,
            kaminari: KaminariMcpCatalog::rack_default(kaminari_device),
            mamori: MamoriPlan::rack_acceptance(public_address, &self.kaminari_host),
        })
    }

    pub fn local_status_url(&self) -> String {
        let host = match self.bind_host.as_str() {
            "0.0.0.0" | "::" => "127.0.0.1",
            host => host,
        };
        format!("http://{host}:{}/status", self.port)
    }

    fn validate(&self) -> Result<(), LaunchConfigError> {
        validate_non_empty("server_name", &self.server_name)?;
        validate_non_empty("bind_host", &self.bind_host)?;
        validate_non_empty("overlay_seed", &self.overlay_seed)?;
        validate_non_empty("kaminari_host", &self.kaminari_host)?;
        validate_non_empty("kaminari_username", &self.kaminari_username)?;
        if self.port == 0 {
            return Err(LaunchConfigError::InvalidPort("port"));
        }
        if self.max_players == 0 {
            return Err(LaunchConfigError::InvalidCount("max_players"));
        }
        if self.tick_rate == 0 {
            return Err(LaunchConfigError::InvalidRate("tick_rate"));
        }
        if self.database_max_connections == 0 {
            return Err(LaunchConfigError::InvalidCount("database_max_connections"));
        }
        if self.anti_ddos_window_seconds == 0 {
            return Err(LaunchConfigError::InvalidCount("anti_ddos_window_seconds"));
        }
        if self.anti_ddos_max_requests == 0 {
            return Err(LaunchConfigError::InvalidCount("anti_ddos_max_requests"));
        }
        if self.game_server_port == 0 {
            return Err(LaunchConfigError::InvalidPort("game_server_port"));
        }
        if let Some(target) = &self.public_dns_target {
            validate_non_empty("public_dns_target", target)?;
        }
        if let Some(sni) = &self.vps_reality_sni {
            validate_non_empty("vps_reality_sni", sni)?;
        }
        if let Some(gmail) = &self.grakane_admin_gmail {
            validate_gmail(gmail)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerEndpoint {
    pub bind_host: String,
    pub port: u16,
    pub local_status_url: String,
    pub public_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmoikaneLaunchManifest {
    pub config: OmoikaneLaunchConfig,
    pub endpoint: ServerEndpoint,
    pub dns: OmoikaneDnsPlan,
    pub publication: OmoikanePublicationPlan,
    pub overlay: Option<OverlayFixedIpProfile>,
    pub kaminari: KaminariMcpCatalog,
    pub mamori: MamoriPlan,
}

impl OmoikaneLaunchManifest {
    pub fn render_terminal(&self) -> String {
        let overlay_ip = self
            .overlay
            .as_ref()
            .map(|overlay| overlay.address.to_string())
            .unwrap_or_else(|| "disabled".to_string());
        let overlay_status = self
            .overlay
            .as_ref()
            .map(OverlayFixedIpProfile::status_url)
            .unwrap_or_else(|| "disabled".to_string());

        let mut out = String::new();
        out.push_str("\x1b[38;5;81m");
        out.push_str("+==============================================================+\n");
        out.push_str("| OMOIKANE SERVER TERMINAL                                     |\n");
        out.push_str("+==============================================================+\n");
        out.push_str("\x1b[0m");
        writeln!(out, " engine        : {}", self.config.server_name)
            .expect("writing terminal to String cannot fail");
        writeln!(
            out,
            " bind          : {}:{}",
            self.endpoint.bind_host, self.endpoint.port
        )
        .expect("writing terminal to String cannot fail");
        writeln!(
            out,
            " tick/max      : {} Hz / {} players",
            self.config.tick_rate, self.config.max_players
        )
        .expect("writing terminal to String cannot fail");
        writeln!(out, " local status  : {}", self.endpoint.local_status_url)
            .expect("writing terminal to String cannot fail");
        writeln!(out, " overlay ip    : {overlay_ip}")
            .expect("writing terminal to String cannot fail");
        writeln!(out, " overlay status: {overlay_status}")
            .expect("writing terminal to String cannot fail");
        writeln!(out, " dns selected  : {}", self.dns.selected_host)
            .expect("writing terminal to String cannot fail");
        writeln!(
            out,
            " public site   : {}",
            self.publication.public_site_url()
        )
        .expect("writing terminal to String cannot fail");
        writeln!(
            out,
            " subservers    : {} published / target {}",
            self.publication.subservers.len(),
            self.publication.target_host
        )
        .expect("writing terminal to String cannot fail");
        writeln!(
            out,
            " grakane gmail : {}",
            self.config
                .grakane_admin_gmail
                .as_deref()
                .map(mask_gmail)
                .unwrap_or_else(|| "not configured".to_string())
        )
        .expect("writing terminal to String cannot fail");
        writeln!(
            out,
            " kaminari profile : {} {}@{}",
            self.kaminari.device.name, self.kaminari.device.username, self.kaminari.device.host
        )
        .expect("writing terminal to String cannot fail");
        writeln!(out, " mamori plan   : {}", self.mamori.name)
            .expect("writing terminal to String cannot fail");
        out.push_str("\x1b[38;5;81m");
        out.push_str("+==============================================================+\n");
        out.push_str("\x1b[0m");
        out
    }

    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_field_name(out, "endpoint", true);
        out.push('{');
        json::push_string_field(out, "bind_host", &self.endpoint.bind_host, true);
        json::push_u16_field(out, "port", self.endpoint.port, false);
        json::push_string_field(
            out,
            "local_status_url",
            &self.endpoint.local_status_url,
            false,
        );
        json::push_string_field(
            out,
            "public_status_url",
            &self.endpoint.public_status_url,
            false,
        );
        out.push('}');
        json::push_field_name(out, "dns", false);
        self.dns.write_json(out);
        json::push_field_name(out, "config", false);
        out.push('{');
        json::push_string_field(out, "server_name", &self.config.server_name, true);
        json::push_string_field(out, "bind_host", &self.config.bind_host, false);
        json::push_u16_field(out, "port", self.config.port, false);
        json::push_usize_field(out, "max_players", self.config.max_players, false);
        json::push_u16_field(out, "tick_rate", self.config.tick_rate, false);
        json::push_string_field(out, "overlay_seed", &self.config.overlay_seed, false);
        json::push_bool_field(out, "overlay_enabled", self.config.overlay_enabled, false);
        json::push_bool_field(
            out,
            "database_enabled",
            self.config.database_url.is_some(),
            false,
        );
        json::push_usize_field(
            out,
            "database_max_connections",
            self.config.database_max_connections as usize,
            false,
        );
        json::push_string_field(out, "kaminari_host", &self.config.kaminari_host, false);
        json::push_string_field(
            out,
            "kaminari_username",
            &self.config.kaminari_username,
            false,
        );
        if let Some(public_dns_name) = &self.config.public_dns_name {
            json::push_string_field(out, "public_dns_name", public_dns_name, false);
        } else {
            json::push_field_name(out, "public_dns_name", false);
            out.push_str("null");
        }
        if let Some(public_dns_target) = &self.config.public_dns_target {
            json::push_string_field(out, "public_dns_target", public_dns_target, false);
        } else {
            json::push_field_name(out, "public_dns_target", false);
            out.push_str("null");
        }
        json::push_bool_field(
            out,
            "grakane_admin_gmail_configured",
            self.config.grakane_admin_gmail.is_some(),
            false,
        );
        json::push_bool_field(
            out,
            "anti_ddos_enabled",
            self.config.anti_ddos_enabled,
            false,
        );
        json::push_usize_field(
            out,
            "anti_ddos_window_seconds",
            self.config.anti_ddos_window_seconds as usize,
            false,
        );
        json::push_usize_field(
            out,
            "anti_ddos_max_requests",
            self.config.anti_ddos_max_requests as usize,
            false,
        );
        json::push_bool_field(
            out,
            "public_site_enabled",
            self.config.public_site_enabled,
            false,
        );
        json::push_bool_field(
            out,
            "game_server_enabled",
            self.config.game_server_enabled,
            false,
        );
        json::push_u16_field(out, "game_server_port", self.config.game_server_port, false);
        json::push_bool_field(out, "vps_mode_enabled", self.config.vps_mode_enabled, false);
        if let Some(vps_reality_sni) = &self.config.vps_reality_sni {
            json::push_string_field(out, "vps_reality_sni", vps_reality_sni, false);
        } else {
            json::push_field_name(out, "vps_reality_sni", false);
            out.push_str("null");
        }
        out.push('}');
        json::push_field_name(out, "publication", false);
        self.publication.write_json(out);
        json::push_field_name(out, "overlay", false);
        if let Some(overlay) = &self.overlay {
            overlay.write_json(out);
        } else {
            out.push_str("null");
        }
        json::push_field_name(out, "kaminari", false);
        self.kaminari.write_json(out);
        json::push_field_name(out, "mamori", false);
        self.mamori.write_json(out);
        out.push('}');
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchConfigError {
    EmptyField(&'static str),
    InvalidGmail(&'static str),
    InvalidPort(&'static str),
    InvalidCount(&'static str),
    InvalidRate(&'static str),
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), LaunchConfigError> {
    if value.trim().is_empty() {
        Err(LaunchConfigError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_gmail(value: &str) -> Result<(), LaunchConfigError> {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    if value.chars().any(char::is_whitespace)
        || value.starts_with('@')
        || !lower.ends_with("@gmail.com")
    {
        return Err(LaunchConfigError::InvalidGmail("grakane_admin_gmail"));
    }
    Ok(())
}

fn mask_gmail(value: &str) -> String {
    let Some((name, domain)) = value.split_once('@') else {
        return "configured".to_string();
    };
    let mut visible = String::new();
    for ch in name.chars().take(2) {
        visible.push(ch);
    }
    if visible.is_empty() {
        visible.push('*');
    }
    format!("{visible}***@{domain}")
}

fn endpoint_host_hint(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(rest) = value.strip_prefix('[') {
        let (host, _) = rest.split_once(']')?;
        return (!host.is_empty()).then(|| host.to_string());
    }
    if value.matches(':').count() == 1 {
        let (host, _) = value.split_once(':')?;
        return (!host.is_empty()).then(|| host.to_string());
    }
    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::OmoikaneLaunchConfig;

    #[test]
    fn launch_manifest_generates_overlay_and_terminal() {
        let config = OmoikaneLaunchConfig {
            server_name: "Omoikane Rack".to_string(),
            port: 9090,
            overlay_seed: "rack-42".to_string(),
            ..OmoikaneLaunchConfig::default()
        };
        let manifest = config.build_manifest().unwrap();

        assert!(manifest.endpoint.local_status_url.ends_with(":9090/status"));
        assert!(manifest.endpoint.public_status_url.contains("100.104."));
        assert!(manifest.overlay.is_some());

        let terminal = manifest.render_terminal();
        assert!(terminal.contains("OMOIKANE SERVER TERMINAL"));
        assert!(terminal.contains("overlay ip"));
        assert!(terminal.contains("public site"));
        assert!(terminal.contains("rack-core"));
    }

    #[test]
    fn launch_manifest_can_disable_overlay() {
        let config = OmoikaneLaunchConfig {
            overlay_enabled: false,
            bind_host: "127.0.0.1".to_string(),
            ..OmoikaneLaunchConfig::default()
        };
        let manifest = config.build_manifest().unwrap();

        assert!(manifest.overlay.is_none());
        assert_eq!(
            manifest.endpoint.public_status_url,
            "http://127.0.0.1:8080/status"
        );
    }

    #[test]
    fn launch_manifest_builds_publication_plan() {
        let config = OmoikaneLaunchConfig {
            public_dns_name: Some("omoikane.example".to_string()),
            public_dns_target: Some("203.0.113.10".to_string()),
            game_server_enabled: true,
            vps_mode_enabled: true,
            vps_reality_sni: Some("front.example".to_string()),
            ..OmoikaneLaunchConfig::default()
        };
        let manifest = config.build_manifest().unwrap();

        assert_eq!(manifest.publication.base_host, "omoikane.example");
        assert!(
            manifest
                .publication
                .dns_records
                .iter()
                .any(|record| record.name == "omoikane.example" && record.value == "203.0.113.10")
        );
        assert!(
            manifest
                .publication
                .subservers
                .iter()
                .any(|server| server.id == "vps-reality" && server.enabled)
        );
    }
}
