use crate::forward;
use crate::http;
use crate::routing::Routing;
use std::io::Write;
use std::net::TcpStream;
use std::sync::Arc;
use std::{io, thread};
use crate::http::StatusCode::{BadGateway, BadRequest};

pub fn handle_connection(mut stream: TcpStream, routing: Arc<Routing>) -> io::Result<()> {

    loop {
        let (headers, end) = http::Request::parse_headers(&mut stream).map_err(|e| {
            match stream.write_all(e.to_status_code().as_bytes()) {
                Ok(_) => io::Error::new(io::ErrorKind::InvalidData, "Protocol Error"),
                Err(e) => io::Error::new(io::ErrorKind::InvalidData, format!("Write Error: {e}")),
            }
        })?;

        let request = http::Request::new(&headers[..end]).ok_or_else(|| {
            match stream.write_all(BadRequest.as_bytes()) {
                Ok(_) => io::Error::new(io::ErrorKind::InvalidData, "Request Error"),
                Err(e) => io::Error::new(io::ErrorKind::InvalidData, format!("Write Error: {e}")),
            }
        })?;

        let addr = routing.select_upstream(&request).ok_or_else(|| {
            match stream.write_all(BadGateway.as_bytes()) {
                Ok(_) => io::Error::new(io::ErrorKind::InvalidData, "Routing Error"),
                Err(e) => io::Error::new(io::ErrorKind::InvalidData, format!("Write Error: {e}")),
            }
        })?;

        let mut upstream = TcpStream::connect(&addr)?;
        upstream.write_all(&headers)?;

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