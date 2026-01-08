use clap::Parser;
use rproxy::{config, connection};
use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_util::task::TaskTracker;

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

    let tracker = TaskTracker::new();

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        let c = Arc::clone(&config);
                        let a = Arc::clone(&args);
                        tracker.spawn(async move {
                            if let Err(e) = connection::handle_connection(stream, c).await {
                                if a.debug {
                                    eprintln!("An error occurred: {}", e);
                                }
                            }
                        });
                    },
                    Err(e) => if args.debug {
                        eprintln!("Accept error: {}", e);
                    },
                }
            },
            _ = tokio::signal::ctrl_c() => {
                if args.debug {
                    eprintln!("\nShutdown signal received...");
                }
                break
            }
        }
    }

    tracker.close();
    if args.debug {
        eprintln!("Waiting for active connections to finish...");
    }

    tracker.wait().await;
    if args.debug {
        eprintln!("Shutdown complete!");
    }

    Ok(())
}
