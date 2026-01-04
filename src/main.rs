use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn forward(buf: &mut [u8], from: &mut TcpStream, to: &mut TcpStream) -> io::Result<usize> {
    from.read(buf).and_then(|bytes| {
        to.write_all(&buf[..bytes]).map(|_| bytes)
    })
}

fn handle_connection(stream: &mut TcpStream) {
    if let Ok(mut upstream) = TcpStream::connect("192.168.124.185:80") {
        let mut buf = [0u8; 8192];

        loop {
            match forward(&mut buf, stream, &mut upstream)
                .and_then(|_| forward(&mut buf, &mut upstream, stream)) {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Error writing to upstream {e}");
                    break;
                }
            }
        }
    }
}

fn main() -> io::Result<()>{
    let listener = TcpListener::bind("localhost:8080")?;
    for maybe_stream in listener.incoming() {
        match maybe_stream {
            Ok(mut stream) => handle_connection(&mut stream),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
