use omoikane_control::OmoikaneLaunchConfig;
use serde::Deserialize;
use std::{fs, io, path::Path};

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct LaunchFileConfig {
    server_name: Option<String>,
    bind_host: Option<String>,
    port: Option<u16>,
    max_players: Option<usize>,
    tick_rate: Option<u16>,
    overlay_seed: Option<String>,
    overlay_enabled: Option<bool>,
    overlay_endpoint_hint: Option<String>,
    database_url: Option<String>,
    database_max_connections: Option<u32>,
    kaminari_host: Option<String>,
    kaminari_username: Option<String>,
}

pub fn load_launch_config_file(
    path: impl AsRef<Path>,
    mut config: OmoikaneLaunchConfig,
) -> io::Result<OmoikaneLaunchConfig> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    let file_config = parse_launch_config_source(path, &source)?;
    apply_file_config(&mut config, file_config);
    Ok(config)
}

fn parse_launch_config_source(path: &Path, source: &str) -> io::Result<LaunchFileConfig> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("json") {
        serde_json::from_str(source).map_err(config_parse_error)
    } else {
        toml::from_str(source).map_err(config_parse_error)
    }
}

fn config_parse_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid Omoikane launch config: {error}"),
    )
}

fn apply_file_config(config: &mut OmoikaneLaunchConfig, file: LaunchFileConfig) {
    if let Some(value) = file.server_name {
        config.server_name = value;
    }
    if let Some(value) = file.bind_host {
        config.bind_host = value;
    }
    if let Some(value) = file.port {
        config.port = value;
    }
    if let Some(value) = file.max_players {
        config.max_players = value;
    }
    if let Some(value) = file.tick_rate {
        config.tick_rate = value;
    }
    if let Some(value) = file.overlay_seed {
        config.overlay_seed = value;
    }
    if let Some(value) = file.overlay_enabled {
        config.overlay_enabled = value;
    }
    if let Some(value) = file.overlay_endpoint_hint {
        config.overlay_endpoint_hint = Some(value);
    }
    if let Some(value) = file.database_url {
        config.database_url = Some(value);
    }
    if let Some(value) = file.database_max_connections {
        config.database_max_connections = value;
    }
    if let Some(value) = file.kaminari_host {
        config.kaminari_host = value;
    }
    if let Some(value) = file.kaminari_username {
        config.kaminari_username = value;
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_file_config, parse_launch_config_source};
    use omoikane_control::OmoikaneLaunchConfig;
    use std::path::Path;

    #[test]
    fn toml_config_overrides_selected_fields() {
        let file = parse_launch_config_source(
            Path::new("omoikane.toml"),
            r#"
server_name = "Omoikane Funcional"
bind_host = "127.0.0.1"
port = 18786
max_players = 96
tick_rate = 120
overlay_enabled = false
kaminari_host = "192.0.2.1"
"#,
        )
        .unwrap();
        let mut config = OmoikaneLaunchConfig::default();
        apply_file_config(&mut config, file);

        assert_eq!(config.server_name, "Omoikane Funcional");
        assert_eq!(config.bind_host, "127.0.0.1");
        assert_eq!(config.port, 18786);
        assert_eq!(config.max_players, 96);
        assert_eq!(config.tick_rate, 120);
        assert!(!config.overlay_enabled);
        assert_eq!(config.kaminari_host, "192.0.2.1");
    }

    #[test]
    fn json_config_accepts_database_and_overlay_fields() {
        let file = parse_launch_config_source(
            Path::new("omoikane.json"),
            r#"{
                "database_url": "postgres://omoikane:secret@db.local/game",
                "database_max_connections": 24,
                "overlay_endpoint_hint": "edge.example:51820"
            }"#,
        )
        .unwrap();
        let mut config = OmoikaneLaunchConfig::default();
        apply_file_config(&mut config, file);

        assert_eq!(config.database_max_connections, 24);
        assert_eq!(
            config.database_url.as_deref(),
            Some("postgres://omoikane:secret@db.local/game")
        );
        assert_eq!(
            config.overlay_endpoint_hint.as_deref(),
            Some("edge.example:51820")
        );
    }
}
