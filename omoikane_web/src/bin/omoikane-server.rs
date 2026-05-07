use actix_web::{App, HttpServer, web};
use daikoku::{DaikokuServer, ServerOptions};
use omoikane_control::OmoikaneLaunchConfig;
use omoikane_web::{OmoikaneActixState, configure_omoikane_routes};
use std::{env, io};

fn main() -> io::Result<()> {
    let config = parse_args(env::args().skip(1))?;
    let manifest = config
        .build_manifest()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, format!("{err:?}")))?;
    let bind_address = format!("{}:{}", config.bind_host, config.port);

    let mut server = DaikokuServer::new(ServerOptions {
        server_name: config.server_name.clone(),
        max_players: config.max_players,
        tick_rate: config.tick_rate,
    });
    server.start();

    println!("{}", manifest.render_terminal());

    let state = web::Data::new(OmoikaneActixState::with_launch_manifest(server, manifest));
    actix_web::rt::System::new().block_on(async move {
        HttpServer::new(move || {
            App::new()
                .app_data(state.clone())
                .configure(configure_omoikane_routes)
        })
        .bind(bind_address)?
        .run()
        .await
    })
}

fn parse_args(args: impl IntoIterator<Item = String>) -> io::Result<OmoikaneLaunchConfig> {
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

fn parse_usize(value: &str, name: &str) -> io::Result<usize> {
    value.parse::<usize>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {name}: {err}"),
        )
    })
}

fn print_help() {
    println!("omoikane-server --name Omoikane --bind 0.0.0.0 --port 8080 --overlay-seed rack-a");
    println!("options:");
    println!("  --name <name>");
    println!("  --bind <host>");
    println!("  --port <port>");
    println!("  --max-players <count>");
    println!("  --tick-rate <hz>");
    println!("  --overlay-seed <seed>");
    println!("  --overlay-endpoint <host:port>");
    println!("  --no-overlay");
    println!("  --junos-host <host>");
    println!("  --junos-user <user>");
}
