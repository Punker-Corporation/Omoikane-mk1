use crate::config::load_launch_config_file;
use crate::sql::OmoikaneSqlState;
use crate::terminal::{render_boot_panel, render_live_panel};
use crate::{OmoikaneHayateState, configure_omoikane_routes};
use actix_web::{App, HttpServer, web};
use daikoku::{DaikokuServer, ServerOptions};
use omoikane_control::OmoikaneLaunchConfig;
use std::env;
use std::io::{self, Write};
use std::net::{TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn run_omoikane_server_from_env() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let interactive = args.is_empty() && env::var_os("OMOIKANE_SKIP_WIZARD").is_none();
    let allow_port_fallback = !args.iter().any(|arg| arg == "--port");
    let mut config = parse_launch_args(args)?;
    if interactive {
        config = launch_interactive_terminal(config)?;
    }
    run_omoikane_server_with_port_fallback(config, allow_port_fallback)
}

pub fn run_omoikane_server(config: OmoikaneLaunchConfig) -> io::Result<()> {
    run_omoikane_server_with_port_fallback(config, false)
}

fn run_omoikane_server_with_port_fallback(
    mut config: OmoikaneLaunchConfig,
    allow_port_fallback: bool,
) -> io::Result<()> {
    if let Some((requested, selected)) = select_runtime_port(&mut config, allow_port_fallback)? {
        eprintln!(
            "Porta {requested} ocupada; iniciando Omoikane em {selected}. Use --port <porta> para fixar uma porta."
        );
    }

    let manifest = config
        .build_manifest()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, format!("{err:?}")))?;
    let bind_address = format!("{}:{}", config.bind_host, config.port);
    let workers = efficient_worker_count();

    let mut server = DaikokuServer::new(ServerOptions {
        server_name: config.server_name.clone(),
        max_players: config.max_players,
        tick_rate: config.tick_rate,
    });
    server.start();
    let shared_server = Arc::new(Mutex::new(server));
    spawn_tick_loop(Arc::clone(&shared_server), config.tick_rate);

    actix_web::rt::System::new().block_on(async move {
        let sql = OmoikaneSqlState::connect_from_config(&config)
            .await
            .map_err(|err| io::Error::other(format!("sqlx connection failed: {err}")))?;
        println!("{}", render_boot_panel(&manifest, sql.as_ref(), workers));

        let state = OmoikaneHayateState::from_shared_with_runtime(shared_server, manifest, sql);
        spawn_terminal_monitor(state.clone());

        let data = web::Data::new(state);
        HttpServer::new(move || {
            App::new()
                .app_data(data.clone())
                .configure(configure_omoikane_routes)
        })
        .workers(workers)
        .bind(bind_address)?
        .run()
        .await
    })
}

fn select_runtime_port(
    config: &mut OmoikaneLaunchConfig,
    allow_port_fallback: bool,
) -> io::Result<Option<(u16, u16)>> {
    match reserve_port(&config.bind_host, config.port) {
        Ok(()) => Ok(None),
        Err(err) if allow_port_fallback && err.kind() == io::ErrorKind::AddrInUse => {
            let requested = config.port;
            for candidate in 8081..=8099 {
                if reserve_port(&config.bind_host, candidate).is_ok() {
                    config.port = candidate;
                    return Ok(Some((requested, candidate)));
                }
            }
            Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "ports 8080..8099 are already in use",
            ))
        }
        Err(err) => Err(err),
    }
}

fn reserve_port(bind_host: &str, port: u16) -> io::Result<()> {
    TcpListener::bind((bind_host, port)).map(drop)
}

pub fn parse_launch_args(
    args: impl IntoIterator<Item = String>,
) -> io::Result<OmoikaneLaunchConfig> {
    let mut config = OmoikaneLaunchConfig::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--config" => {
                config = load_launch_config_file(next_value(&mut args, "--config")?, config)?
            }
            "--name" => config.server_name = next_value(&mut args, "--name")?,
            "--bind" => config.bind_host = next_value(&mut args, "--bind")?,
            "--port" => config.port = parse_u16(&next_value(&mut args, "--port")?, "--port")?,
            "--max-players" => {
                config.max_players =
                    parse_usize(&next_value(&mut args, "--max-players")?, "--max-players")?
            }
            "--tick-rate" => {
                config.tick_rate = parse_u16(&next_value(&mut args, "--tick-rate")?, "--tick-rate")?
            }
            "--overlay-seed" => config.overlay_seed = next_value(&mut args, "--overlay-seed")?,
            "--overlay-endpoint" => {
                config.overlay_endpoint_hint = Some(next_value(&mut args, "--overlay-endpoint")?)
            }
            "--no-overlay" => config.overlay_enabled = false,
            "--database-url" => {
                config.database_url = Some(next_value(&mut args, "--database-url")?)
            }
            "--database-max-connections" => {
                config.database_max_connections = parse_u32(
                    &next_value(&mut args, "--database-max-connections")?,
                    "--database-max-connections",
                )?
            }
            "--kaminari-host" => config.kaminari_host = next_value(&mut args, "--kaminari-host")?,
            "--kaminari-user" => {
                config.kaminari_username = next_value(&mut args, "--kaminari-user")?
            }
            "--public-dns" => config.public_dns_name = Some(next_value(&mut args, "--public-dns")?),
            "--grakane-admin-gmail" => {
                config.grakane_admin_gmail = Some(next_value(&mut args, "--grakane-admin-gmail")?)
            }
            "--anti-ddos" => config.anti_ddos_enabled = true,
            "--no-anti-ddos" => config.anti_ddos_enabled = false,
            "--anti-ddos-window" => {
                config.anti_ddos_window_seconds = parse_u16(
                    &next_value(&mut args, "--anti-ddos-window")?,
                    "--anti-ddos-window",
                )?
            }
            "--anti-ddos-max-requests" => {
                config.anti_ddos_max_requests = parse_u32(
                    &next_value(&mut args, "--anti-ddos-max-requests")?,
                    "--anti-ddos-max-requests",
                )?
            }
            unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {unknown}"),
                ));
            }
        }
    }
    Ok(config)
}

fn launch_interactive_terminal(
    mut config: OmoikaneLaunchConfig,
) -> io::Result<OmoikaneLaunchConfig> {
    println!("\x1b[2J\x1b[H\x1b[38;5;81mOmoikane Mikado Terminal\x1b[0m");
    println!("Selecione o IP que vai receber o servidor.");
    let hosts = available_bind_hosts(&config);
    for (index, host) in hosts.iter().enumerate() {
        println!("  [{index}] {host}");
    }
    let selected_host = read_index("IP", hosts.len(), 0)?;
    config.bind_host = hosts[selected_host].clone();

    let preview = config
        .build_manifest()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, format!("{err:?}")))?;
    println!();
    println!("Selecione o DNS/identidade publica para substituir o IP nos links.");
    for (index, choice) in preview.dns.choices.iter().enumerate() {
        println!(
            "  [{index}] {} -> {} ({}, ttl={}s)",
            choice.label, choice.host, choice.record_type, choice.ttl_seconds
        );
    }
    let selected_dns = read_index("DNS", preview.dns.choices.len(), 0)?;
    config.public_dns_name = Some(preview.dns.choices[selected_dns].host.clone());

    println!();
    println!("Informe o Gmail administrador do Grakane nesta maquina host.");
    loop {
        let gmail = read_line("gmail")?;
        if is_valid_gmail(&gmail) {
            config.grakane_admin_gmail = Some(gmail.trim().to_ascii_lowercase());
            break;
        }
        println!("Use um endereco @gmail.com valido para destravar o Grakane.");
    }

    println!();
    println!(
        "Firewall logico anti-DDoS: {} requisicoes/{}s",
        config.anti_ddos_max_requests, config.anti_ddos_window_seconds
    );
    println!("Iniciando servidor Omoikane...");
    Ok(config)
}

fn available_bind_hosts(config: &OmoikaneLaunchConfig) -> Vec<String> {
    let mut hosts = Vec::new();
    push_unique(&mut hosts, config.bind_host.clone());
    push_unique(&mut hosts, "127.0.0.1".to_string());
    push_unique(&mut hosts, "0.0.0.0".to_string());
    if let Some(primary) = primary_lan_ip() {
        push_unique(&mut hosts, primary);
    }
    hosts
}

fn primary_lan_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn read_index(label: &str, len: usize, default: usize) -> io::Result<usize> {
    loop {
        let line = read_line(&format!("{label} [{default}]"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(default.min(len.saturating_sub(1)));
        }
        if let Ok(index) = trimmed.parse::<usize>() {
            if index < len {
                return Ok(index);
            }
        }
        println!("Escolha um numero entre 0 e {}.", len.saturating_sub(1));
    }
}

fn read_line(label: &str) -> io::Result<String> {
    print!("{label}> ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line)
}

fn is_valid_gmail(value: &str) -> bool {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    !value.is_empty()
        && !value.starts_with('@')
        && !value.chars().any(char::is_whitespace)
        && lower.ends_with("@gmail.com")
}

fn spawn_tick_loop(server: Arc<Mutex<DaikokuServer>>, tick_rate: u16) {
    let tick_duration = Duration::from_secs_f32(1.0 / f32::from(tick_rate.max(1)));
    thread::spawn(move || {
        loop {
            let started = std::time::Instant::now();
            if let Ok(mut server) = server.lock() {
                server.tick_update(tick_duration.as_secs_f32());
            }
            let elapsed = started.elapsed();
            if elapsed < tick_duration {
                thread::sleep(tick_duration - elapsed);
            }
        }
    });
}

fn spawn_terminal_monitor(state: OmoikaneHayateState) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            if let Ok(snapshot) = state.status_snapshot() {
                println!(
                    "{}",
                    render_live_panel(
                        &state.launch_manifest(),
                        &snapshot,
                        &state.metrics(),
                        state.sql().as_ref()
                    )
                );
            }
        }
    });
}

fn efficient_worker_count() -> usize {
    std::thread::available_parallelism()
        .map_or(1, usize::from)
        .clamp(1, 16)
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> io::Result<String> {
    args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("missing value for {name}"),
        )
    })
}

fn parse_u16(value: &str, name: &str) -> io::Result<u16> {
    value.parse::<u16>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {name}: {err}"),
        )
    })
}

fn parse_u32(value: &str, name: &str) -> io::Result<u32> {
    value.parse::<u32>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {name}: {err}"),
        )
    })
}

fn parse_usize(value: &str, name: &str) -> io::Result<usize> {
    value.parse::<usize>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {name}: {err}"),
        )
    })
}

fn print_help() {
    println!("omoikane --name Omoikane --bind 0.0.0.0 --port 8080 --overlay-seed rack-a");
    println!("options:");
    println!("  --config <omoikane.toml|omoikane.json>");
    println!("  --name <name>");
    println!("  --bind <host>");
    println!("  --port <port>");
    println!("  --max-players <count>");
    println!("  --tick-rate <hz>");
    println!("  --overlay-seed <seed>");
    println!("  --overlay-endpoint <host:port>");
    println!("  --no-overlay");
    println!("  --database-url <postgres-url>");
    println!("  --database-max-connections <count>");
    println!("  --kaminari-host <host>");
    println!("  --kaminari-user <user>");
    println!("  --public-dns <host>");
    println!("  --grakane-admin-gmail <gmail>");
    println!("  --anti-ddos | --no-anti-ddos");
    println!("  --anti-ddos-window <seconds>");
    println!("  --anti-ddos-max-requests <count>");
    println!("double-click:");
    println!("  without arguments, Omoikane opens the IP/DNS/Gmail terminal before launch");
    println!("  without --port, Omoikane uses 8080 or the first free port from 8081..8099");
    println!("endpoints:");
    println!("  / /console /status /launch /metrics /grakane/dashboard.json /database/status");
    println!("  /automation/mamori /automation/kaminari/tools /automation/michisuji/rb2011.rsc");
}

#[cfg(test)]
mod tests {
    use super::{parse_launch_args, select_runtime_port};
    use omoikane_control::OmoikaneLaunchConfig;
    use std::net::TcpListener;

    #[test]
    fn launch_args_parse_sql_and_overlay_options() {
        let config = parse_launch_args(
            [
                "--name",
                "Omoikane Final",
                "--database-url",
                "postgres://omoikane:secret@db.local/game",
                "--database-max-connections",
                "32",
                "--public-dns",
                "rack.example",
                "--grakane-admin-gmail",
                "host@gmail.com",
                "--anti-ddos-max-requests",
                "1200",
                "--no-overlay",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .unwrap();

        assert_eq!(config.server_name, "Omoikane Final");
        assert_eq!(config.database_max_connections, 32);
        assert!(config.database_url.is_some());
        assert!(!config.overlay_enabled);
        assert_eq!(config.public_dns_name.as_deref(), Some("rack.example"));
        assert_eq!(
            config.grakane_admin_gmail.as_deref(),
            Some("host@gmail.com")
        );
        assert_eq!(config.anti_ddos_max_requests, 1200);
    }

    #[test]
    fn default_port_falls_back_when_busy() {
        let _guard = TcpListener::bind(("127.0.0.1", 8080)).ok();
        let mut config = OmoikaneLaunchConfig {
            bind_host: "127.0.0.1".to_string(),
            port: 8080,
            ..OmoikaneLaunchConfig::default()
        };

        let selected = select_runtime_port(&mut config, true).unwrap();

        assert!(matches!(selected, Some((8080, 8081..=8099))));
        assert!((8081..=8099).contains(&config.port));
    }
}
