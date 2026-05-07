use crate::sql::OmoikaneSqlState;
use crate::terminal::{render_boot_panel, render_live_panel};
use crate::{OmoikaneActixState, configure_omoikane_routes};
use actix_web::{App, HttpServer, web};
use daikoku::{DaikokuServer, ServerOptions};
use omoikane_control::OmoikaneLaunchConfig;
use std::env;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn run_omoikane_server_from_env() -> io::Result<()> {
    let config = parse_launch_args(env::args().skip(1))?;
    run_omoikane_server(config)
}

pub fn run_omoikane_server(config: OmoikaneLaunchConfig) -> io::Result<()> {
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

        let state = OmoikaneActixState::from_shared_with_runtime(shared_server, manifest, sql);
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
            "--junos-host" => config.junos_host = next_value(&mut args, "--junos-host")?,
            "--junos-user" => config.junos_username = next_value(&mut args, "--junos-user")?,
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

fn spawn_terminal_monitor(state: OmoikaneActixState) {
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
    println!("  --junos-host <host>");
    println!("  --junos-user <user>");
    println!("endpoints:");
    println!("  /status /launch /metrics /grafana/dashboard.json /database/status");
}

#[cfg(test)]
mod tests {
    use super::parse_launch_args;

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
    }
}
