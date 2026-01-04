use crate::http;
use crate::routing::Routing;
use crate::forward;
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn handle_connection(stream: &mut TcpStream, routing: &Routing) -> io::Result<()> {
    let mut head = [0u8; 8192];
    let mut buf = [0u8; 8192];

    if let Ok(bytes) = stream.read(&mut head) {

        let request = http::Request::new(&mut head).expect("Expected Request");
        let addr = routing.select_upstream(&request).expect(format!("Route not found {:?}", request).as_str());

        if let Ok(mut upstream) = TcpStream::connect(&addr) {

            upstream.write_all(&mut head)?;
            if bytes < head.len() {
                forward::forward(&mut buf, &mut upstream, stream)?;
            }

            loop {
                match forward::forward(&mut buf, stream, &mut upstream)
                    .and_then(|_| forward::forward(&mut buf, &mut upstream, stream)) {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Error writing to upstream {e}");
                        break;
                    }
                }
            }
        } else {
            eprintln!("Failed to connect to {addr}");
        }
    }
    Ok(())
}