use std::io;

fn main() -> io::Result<()> {
    omoikane_web::run_omoikane_server_from_env()
}
