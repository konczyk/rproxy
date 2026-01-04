use std::{io, thread};
use std::net::TcpListener;
use std::sync::Arc;

mod forward;
mod connection;
mod http;
mod routing;

fn main() -> io::Result<()>{
    let listener = TcpListener::bind("localhost:8080")?;
    let routing = Arc::from(routing::Routing::new());
    for maybe_stream in listener.incoming() {
        match maybe_stream {
            Ok(stream) => {
                let r = Arc::clone(&routing);
                thread::spawn(move || {
                    match connection::handle_connection(stream, r) {
                        Ok(_) => (),
                        Err(e) => eprintln!("Couldn't handle connection {e}"),
                    }
                });
            },
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
