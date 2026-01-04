use std::io;
use std::io::Read;
use crate::forward;
use crate::http;
use std::net::TcpStream;

pub fn handle_connection(stream: &mut TcpStream) -> io::Result<()> {
    let mut buf = [0u8; 8192];
    stream.read(&mut buf)?;

    let _request = http::Request::new(&mut buf).expect("Expected Request");

    if let Ok(mut upstream) = TcpStream::connect("192.168.124.185:80") {

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
    Ok(())
}