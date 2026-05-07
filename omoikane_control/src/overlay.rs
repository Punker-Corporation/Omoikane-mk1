use crate::json;
use std::fmt::Write as _;
use std::net::Ipv4Addr;

pub const OMOIKANE_OVERLAY_ROOT: Ipv4Addr = Ipv4Addr::new(100, 104, 0, 0);
pub const OMOIKANE_OVERLAY_PREFIX: u8 = 16;
pub const DEFAULT_OVERLAY_LISTEN_PORT: u16 = 51820;
pub const DEFAULT_OVERLAY_KEEPALIVE_SECONDS: u16 = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayFixedIpProfile {
    pub service_name: String,
    pub seed: String,
    pub address: Ipv4Addr,
    pub prefix: u8,
    pub service_port: u16,
    pub listen_port: u16,
    pub private_key: String,
    pub public_key: String,
    pub endpoint_hint: Option<String>,
    pub persistent_keepalive_seconds: u16,
}

impl OverlayFixedIpProfile {
    pub fn omoikane(
        service_name: impl Into<String>,
        service_port: u16,
        seed: impl Into<String>,
    ) -> Self {
        let service_name = service_name.into();
        let seed = seed.into();
        Self {
            address: stable_overlay_ip(&seed, &service_name),
            service_name,
            seed,
            prefix: OMOIKANE_OVERLAY_PREFIX,
            service_port,
            listen_port: DEFAULT_OVERLAY_LISTEN_PORT,
            private_key: "<omoikane-server-private-key>".to_string(),
            public_key: "<omoikane-server-public-key>".to_string(),
            endpoint_hint: None,
            persistent_keepalive_seconds: DEFAULT_OVERLAY_KEEPALIVE_SECONDS,
        }
    }

    pub fn with_endpoint_hint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint_hint = Some(endpoint.into());
        self
    }

    pub fn with_keys(
        mut self,
        private_key: impl Into<String>,
        public_key: impl Into<String>,
    ) -> Self {
        self.private_key = private_key.into();
        self.public_key = public_key.into();
        self
    }

    pub fn with_listen_port(mut self, listen_port: u16) -> Self {
        self.listen_port = listen_port;
        self
    }

    pub fn status_url(&self) -> String {
        format!("http://{}:{}/status", self.address, self.service_port)
    }

    pub fn render_server_config(&self) -> Result<String, OverlayConfigError> {
        self.validate()?;

        let mut config = String::new();
        config.push_str("# Omoikane fixed overlay endpoint\n");
        config.push_str("# Replace key placeholders before applying to a real WireGuard device.\n");
        writeln!(config, "[Interface]").expect("writing config to String cannot fail");
        writeln!(config, "Address = {}/32", self.address)
            .expect("writing config to String cannot fail");
        writeln!(config, "ListenPort = {}", self.listen_port)
            .expect("writing config to String cannot fail");
        writeln!(config, "PrivateKey = {}", self.private_key)
            .expect("writing config to String cannot fail");
        writeln!(config).expect("writing config to String cannot fail");
        writeln!(config, "# service = {}", self.service_name)
            .expect("writing config to String cannot fail");
        writeln!(config, "# status = {}", self.status_url())
            .expect("writing config to String cannot fail");
        Ok(config)
    }

    pub fn render_client_peer_config(&self) -> Result<String, OverlayConfigError> {
        self.validate()?;

        let endpoint = self
            .endpoint_hint
            .as_deref()
            .unwrap_or("<omoikane-overlay-entrypoint>:51820");

        let mut config = String::new();
        config.push_str("# Omoikane peer profile for clients and rack routers\n");
        writeln!(config, "[Peer]").expect("writing config to String cannot fail");
        writeln!(config, "PublicKey = {}", self.public_key)
            .expect("writing config to String cannot fail");
        writeln!(config, "AllowedIPs = {}/32", self.address)
            .expect("writing config to String cannot fail");
        writeln!(config, "Endpoint = {endpoint}").expect("writing config to String cannot fail");
        writeln!(
            config,
            "PersistentKeepalive = {}",
            self.persistent_keepalive_seconds
        )
        .expect("writing config to String cannot fail");
        Ok(config)
    }

    pub fn write_json(&self, out: &mut String) {
        out.push('{');
        json::push_string_field(out, "service_name", &self.service_name, true);
        json::push_string_field(out, "seed", &self.seed, false);
        json::push_string_field(out, "address", &self.address.to_string(), false);
        json::push_u16_field(out, "prefix", self.prefix as u16, false);
        json::push_u16_field(out, "service_port", self.service_port, false);
        json::push_u16_field(out, "listen_port", self.listen_port, false);
        json::push_string_field(out, "status_url", &self.status_url(), false);
        json::push_field_name(out, "endpoint_hint", false);
        if let Some(endpoint_hint) = &self.endpoint_hint {
            json::push_string(out, endpoint_hint);
        } else {
            out.push_str("null");
        }
        json::push_u16_field(
            out,
            "persistent_keepalive_seconds",
            self.persistent_keepalive_seconds,
            false,
        );
        out.push('}');
    }

    fn validate(&self) -> Result<(), OverlayConfigError> {
        validate_non_empty("service_name", &self.service_name)?;
        validate_non_empty("seed", &self.seed)?;
        validate_non_empty("private_key", &self.private_key)?;
        validate_non_empty("public_key", &self.public_key)?;
        if self.prefix == 0 || self.prefix > 32 {
            return Err(OverlayConfigError::InvalidPrefix(self.prefix));
        }
        if self.service_port == 0 {
            return Err(OverlayConfigError::InvalidPort("service_port"));
        }
        if self.listen_port == 0 {
            return Err(OverlayConfigError::InvalidPort("listen_port"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayConfigError {
    EmptyField(&'static str),
    InvalidPrefix(u8),
    InvalidPort(&'static str),
}

pub fn stable_overlay_ip(seed: &str, service_name: &str) -> Ipv4Addr {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in seed.bytes().chain([0xff]).chain(service_name.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    let third = ((hash >> 8) & 0xff) as u8;
    let fourth = ((hash & 0xff) as u8 % 254) + 1;
    Ipv4Addr::new(100, 104, third, fourth)
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), OverlayConfigError> {
    if value.trim().is_empty() {
        Err(OverlayConfigError::EmptyField(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{OverlayFixedIpProfile, stable_overlay_ip};

    #[test]
    fn overlay_ip_is_stable_and_service_scoped() {
        let left = stable_overlay_ip("rack-a", "Omoikane");
        let right = stable_overlay_ip("rack-a", "Omoikane");
        let other = stable_overlay_ip("rack-a", "Omoikane staging");

        assert_eq!(left, right);
        assert_ne!(left, other);
        assert_eq!(left.octets()[0], 100);
        assert_eq!(left.octets()[1], 104);
    }

    #[test]
    fn overlay_profile_renders_wireguard_style_configs() {
        let profile = OverlayFixedIpProfile::omoikane("Omoikane", 8080, "rack-a")
            .with_endpoint_hint("203.0.113.10:51820")
            .with_keys("server-private", "server-public");

        let server = profile.render_server_config().unwrap();
        assert!(server.contains("[Interface]"));
        assert!(server.contains("PrivateKey = server-private"));
        assert!(server.contains("status = http://"));

        let peer = profile.render_client_peer_config().unwrap();
        assert!(peer.contains("[Peer]"));
        assert!(peer.contains("PublicKey = server-public"));
        assert!(peer.contains("Endpoint = 203.0.113.10:51820"));
        assert!(!peer.contains("Activate"));
    }
}
