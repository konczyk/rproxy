use crate::forward;
use std::net::TcpStream;

pub fn handle_connection(stream: &mut TcpStream) {
    if let Ok(mut upstream) = TcpStream::connect("192.168.124.185:80") {
        let mut buf = [0u8; 8192];

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
    }
}