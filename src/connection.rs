use std::io;
use std::io::{Read, Write};
use crate::forward;
use crate::http;
use std::net::TcpStream;

pub fn handle_connection(stream: &mut TcpStream) -> io::Result<()> {
    let mut head = [0u8; 8192];
    let mut buf = [0u8; 8192];
    
    if let Ok(bytes) = stream.read(&mut head) {
        let _request = http::Request::new(&mut head).expect("Expected Request");

        if let Ok(mut upstream) = TcpStream::connect("192.168.124.185:80") {

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
        }
    }
    Ok(())
}