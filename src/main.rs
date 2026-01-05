use clap::Parser;
use rproxy::connection;
use rproxy::routing;
use std::net::TcpListener;
use std::sync::Arc;
use std::{io, thread};
use rproxy::routing::Route;

#[derive(Parser)]
struct Args {

    /// display debugging information
    #[arg(short, long)]
    debug: bool,
}

fn main() -> io::Result<()>{
    let args = Arc::from(Args::parse());
    let listener = TcpListener::bind("localhost:8080")?;

    let routes = vec![Route {
        host: "localhost:8080".as_bytes().to_vec(),
        path: "/pl".as_bytes().to_vec(),
        addr: "192.168.124.185:80".to_string()
    }];
    let routing = Arc::from(routing::Routing::new(routes));
    for maybe_stream in listener.incoming() {
        match maybe_stream {
            Ok(stream) => {
                let r = Arc::clone(&routing);
                let a = Arc::clone(&args);

                thread::spawn(move || {
                    match connection::handle_connection(stream, r) {
                        Ok(_) => (),
                        Err(e) => if a.debug {
                            eprintln!("Connection failed: {e}")
                        },
                    }
                });
            },
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
