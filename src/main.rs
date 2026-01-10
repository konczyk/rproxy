use clap::Parser;
use rproxy::{config, connection};
use std::sync::Arc;
use std::{env, io};
use tokio::net::TcpListener;
use tokio_util::task::TaskTracker;
use tracing::{error, info, warn};

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
    let args = Args::parse();
    let config = Arc::new(config::Config::new(&args.config)?);
    let listener = TcpListener::bind(&config.listen).await?;
    let filter = env::var("RUST_LOG").unwrap_or_else(|_| (if args.debug { "debug" } else { "info" }).to_string());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    let tracker = TaskTracker::new();

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        let c = Arc::clone(&config);
                        tracker.spawn(async move {
                            let _ = connection::handle_connection(stream, c).await;
                        });
                    },
                    Err(e) => error!("Accept error: {}", e)
                }
            },
            _ = tokio::signal::ctrl_c() => {
                warn!("\nShutdown signal received...");
                break
            }
        }
    }

    tracker.close();
    info!("Waiting for active connections to finish...");

    tracker.wait().await;
    info!("Shutdown complete!");

    Ok(())
}
