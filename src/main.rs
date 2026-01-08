use clap::Parser;
use rproxy::routing::Route;
use rproxy::{config, connection};
use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Parser)]
struct Args {

    /// Run in a debug mode
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> io::Result<()>{
    let args = Arc::from(Args::parse());
    let listener = TcpListener::bind("localhost:8080").await?;

    let routes = vec![Route {
        host: "localhost:8080".as_bytes().to_vec(),
        path: "/pl".as_bytes().to_vec(),
        addr: "192.168.124.185:80".to_string()
    }];
    let routing = Arc::from(config::Config::new(routes));

    loop {
        let (stream, _) = listener.accept().await?;
        let r = Arc::clone(&routing);
        let a = Arc::clone(&args);

        tokio::spawn(async move {
            if let Err(e) = connection::handle_connection(stream, r).await {
                if a.debug {
                    eprintln!("Connection error: {}", e);
                }
            }
        });
    }
}
