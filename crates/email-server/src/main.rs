use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(env, long, default_value = "0.0.0.0:25")]
    smtp_listen_address: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    email_server_core::smtp_server(&*args.smtp_listen_address)
        .await
        .unwrap();
}
