use std::io;
use std::net::TcpListener;

mod forward;

mod connection;

fn main() -> io::Result<()>{
    let listener = TcpListener::bind("localhost:8080")?;
    for maybe_stream in listener.incoming() {
        match maybe_stream {
            Ok(mut stream) => connection::handle_connection(&mut stream),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
