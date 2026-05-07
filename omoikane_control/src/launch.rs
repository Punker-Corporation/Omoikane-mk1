use crate::json;
use crate::junos::{JunosDeviceProfile, JunosMcpCatalog};
use crate::overlay::OverlayFixedIpProfile;
use crate::robot::NetworkRobotPlan;
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
    pub junos_host: String,
    pub junos_username: String,
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
            junos_host: "192.168.88.1".to_string(),
            junos_username: "netops".to_string(),
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
        let junos_device =
            JunosDeviceProfile::rack_default("rack-core", &self.junos_host, &self.junos_username);

        Ok(OmoikaneLaunchManifest {
            endpoint: ServerEndpoint {
                bind_host: self.bind_host.clone(),
                port: self.port,
                local_status_url: self.local_status_url(),
                public_status_url: format!("http://{public_address}:{}/status", self.port),
            },
            config: self.clone(),
            overlay,
            junos: JunosMcpCatalog::rack_default(junos_device),
            robot: NetworkRobotPlan::rack_acceptance(public_address, &self.junos_host),
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
        validate_non_empty("junos_host", &self.junos_host)?;
        validate_non_empty("junos_username", &self.junos_username)?;
        if self.port == 0 {
            return Err(LaunchConfigError::InvalidPort("port"));
        }
        if self.max_players == 0 {
            return Err(LaunchConfigError::InvalidCount("max_players"));
        }
        if self.tick_rate == 0 {
            return Err(LaunchConfigError::InvalidRate("tick_rate"));
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
    pub overlay: Option<OverlayFixedIpProfile>,
    pub junos: JunosMcpCatalog,
    pub robot: NetworkRobotPlan,
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
        writeln!(
            out,
            " junos profile : {} {}@{}",
            self.junos.device.name, self.junos.device.username, self.junos.device.host
        )
        .expect("writing terminal to String cannot fail");
        writeln!(out, " robot plan    : {}", self.robot.name)
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
        json::push_field_name(out, "config", false);
        out.push('{');
        json::push_string_field(out, "server_name", &self.config.server_name, true);
        json::push_string_field(out, "bind_host", &self.config.bind_host, false);
        json::push_u16_field(out, "port", self.config.port, false);
        json::push_usize_field(out, "max_players", self.config.max_players, false);
        json::push_u16_field(out, "tick_rate", self.config.tick_rate, false);
        json::push_string_field(out, "overlay_seed", &self.config.overlay_seed, false);
        json::push_bool_field(out, "overlay_enabled", self.config.overlay_enabled, false);
        json::push_string_field(out, "junos_host", &self.config.junos_host, false);
        json::push_string_field(out, "junos_username", &self.config.junos_username, false);
        out.push('}');
        json::push_field_name(out, "overlay", false);
        if let Some(overlay) = &self.overlay {
            overlay.write_json(out);
        } else {
            out.push_str("null");
        }
        json::push_field_name(out, "junos", false);
        self.junos.write_json(out);
        json::push_field_name(out, "robot", false);
        self.robot.write_json(out);
        out.push('}');
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchConfigError {
    EmptyField(&'static str),
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
}
