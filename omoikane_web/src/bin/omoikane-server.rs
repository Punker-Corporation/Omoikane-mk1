use std::io;

fn main() {
    if let Err(err) = omoikane_web::run_omoikane_server_from_env() {
        eprintln!();
        eprintln!("Omoikane nao conseguiu iniciar o servidor: {err}");
        eprintln!("Verifique se a porta esta livre ou use --port <porta> para escolher outra.");
        pause_for_double_click();
        std::process::exit(1);
    }
}

fn pause_for_double_click() {
    if std::env::args_os().len() > 1 {
        return;
    }

    eprintln!();
    eprintln!("Pressione Enter para fechar esta janela.");
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}
