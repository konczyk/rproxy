use clap::Parser;
use rproxy::{config, connection};
use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Parser)]
struct Args {

    /// Config file to load
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Run in a debug mode
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> io::Result<()>{
    let args = Arc::from(Args::parse());
    let config = config::Config::new(&args.config)?;
    let listener = TcpListener::bind(&config.listen).await?;

    let config = Arc::new(config);

    loop {
        let (stream, _) = listener.accept().await?;
        let c = Arc::clone(&config);
        let a = Arc::clone(&args);

        tokio::spawn(async move {
            if let Err(e) = connection::handle_connection(stream, c).await {
                if a.debug {
                    eprintln!("Connection error: {}", e);
                }
            }
        });
    }
}
