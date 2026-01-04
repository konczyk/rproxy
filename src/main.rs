extern crate core;

use std::io;
use std::net::TcpListener;

mod forward;
mod connection;
mod http;
mod routing;

fn main() -> io::Result<()>{
    let listener = TcpListener::bind("localhost:8080")?;
    let routing = routing::Routing::new();
    for maybe_stream in listener.incoming() {
        match maybe_stream {
            Ok(mut stream) => {
                match connection::handle_connection(stream, &routing) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Couldn't handle connection {e}"),
                }
            },
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
