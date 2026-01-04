use crate::forward;
use crate::http;
use crate::routing::Routing;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::{io, thread};

pub fn handle_connection(mut stream: TcpStream, routing: Arc<Routing>) -> io::Result<()> {
    let mut hbuf = [0u8; 512];
    let mut head = Vec::with_capacity(8192);

    loop {
        let mut end = head.len();
        if let Ok(bytes) = stream.read(&mut hbuf) {
            head.extend_from_slice(&hbuf[..bytes]);
            if head.len() >= 64*1024 {
                return Err(io::Error::new(io::ErrorKind::Other, "Headers too large"));
            }
            let (first, last) = head.split_at(head.len() - 1024.min(head.len()));
            for (i, ch) in last.iter().enumerate() {
                if *ch == b'\r' && last.len() >= i + 4 {
                    if &last[i..=i+3] == b"\r\n\r\n" {
                        end = first.len() + i + 4;
                        break;
                    }

                }
            }
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "Connection reset"));
        }

        if end == 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Error reading HTTP header"));
        }

        let request = match http::Request::new(&head[..end]) {
            Some(r) => r,
            None => return Err(io::Error::new(io::ErrorKind::Other, "Invalid request"))
        };
        let addr = routing.select_upstream(&request).expect(format!("Route not found {:?}", request).as_str());

        let mut upstream = TcpStream::connect(&addr)?;
        upstream.write_all(&head)?;

        let mut c_stream = stream.try_clone().expect("Stream cloning failed");
        let mut c_upstream = upstream.try_clone().expect("Stream cloning failed");

        let client_handle = thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match forward::forward(&mut buf, &mut c_stream, &mut c_upstream) {
                    Ok(0) => break,
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Error writing to upstream {e}");
                        break;
                    }
                }
            }
        });

        let mut u_stream = stream.try_clone().expect("Stream cloning failed");
        let mut u_upstream = upstream.try_clone().expect("Stream cloning failed");

        let upstream_handle = thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match forward::forward(&mut buf, &mut u_upstream, &mut u_stream) {
                    Ok(0) => break,
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Error reading from upstream {e}");
                        break;
                    }
                }
            }
        });

        client_handle.join().unwrap();
        upstream_handle.join().unwrap();

        break;
    }

    Ok(())
}