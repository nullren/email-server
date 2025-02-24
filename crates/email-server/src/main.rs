use clap::Parser;
use email_server_core::logging;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(env, long, default_value = "0.0.0.0:25")]
    smtp_listen_address: String,

    #[arg(env, long, default_value = "email.db")]
    sqlite_path: String,
}

#[tokio::main]
async fn main() {
    logging::setup();
    let args = Args::parse();

    email_server_core::smtp_server(&*args.smtp_listen_address, &*args.sqlite_path)
        .await
        .unwrap();
}
