use crate::config::Config;
use crate::http;
use crate::http::StatusCode::{BadGateway, BadRequest, GatewayTimeout, NotFound};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{copy_bidirectional, AsyncWriteExt};
use tokio::net::TcpStream;
use io::ErrorKind;

pub async fn handle_connection(mut stream: TcpStream, config: Arc<Config>) -> io::Result<()> {

    let (headers, end) = match http::Request::parse_headers(&mut stream).await {
        Ok(val) => Ok(val),
        Err(e) => Err(
            match stream.write_all(e.to_status_code().as_bytes()).await {
                Ok(_) => io::Error::new(ErrorKind::InvalidData, "Protocol Error"),
                Err(e) => io::Error::new(ErrorKind::InvalidData, format!("Write Error: {e}")),
            }),
    }?;

    let request = match http::Request::new(&headers[..end]) {
        Some(req) => Ok(req),
        None => Err(
            match stream.write_all(BadRequest.as_bytes()).await {
                Ok(_) => io::Error::new(ErrorKind::InvalidData, "Request Error"),
                Err(e) => io::Error::new(ErrorKind::InvalidData, format!("Write Error: {e}")),
            }),
    }?;

    let route = match config.select_upstream(&request) {
        Some(a) => Ok(a),
        None => Err(
            match stream.write_all(NotFound.as_bytes()).await {
                Ok(_) => io::Error::new(ErrorKind::InvalidData, "Routing Error"),
                Err(e) => io::Error::new(ErrorKind::InvalidData, format!("Write Error: {e}")),
            }),
    }?;

    let timeout = Duration::from_millis(route.timeout.unwrap_or(10_000));

    let result = tokio::time::timeout(timeout, async {
        let mut upstream = TcpStream::connect(&route.addr).await?;
        upstream.write_all(&headers).await?;

        copy_bidirectional(&mut stream, &mut upstream).await
    }).await;

    match result {
        Ok(Ok(_)) => {
            Ok(())
        },
        Ok(Err(e)) => Err(
            match e.kind() {
                ErrorKind::ConnectionRefused | ErrorKind::AddrNotAvailable | ErrorKind::Other => {
                    match stream.write_all(BadGateway.as_bytes()).await {
                        Ok(_) => io::Error::new(ErrorKind::InvalidData, format!("Connection Error: {e}")),
                        Err(e) => io::Error::new(ErrorKind::InvalidData, format!("Write Error: {e}")),
                    }
                },
                ErrorKind::TimedOut => {
                    match stream.write_all(GatewayTimeout.as_bytes()).await {
                        Ok(_) => io::Error::new(ErrorKind::InvalidData, format!("Connection Error: {e}")),
                        Err(e) => io::Error::new(ErrorKind::InvalidData, format!("Write Error: {e}")),
                    }
                },
                _ => {
                    match stream.write_all(BadRequest.as_bytes()).await {
                        Ok(_) => io::Error::new(ErrorKind::InvalidData, format!("Tunnel Error: {e}")),
                        Err(e) => io::Error::new(ErrorKind::InvalidData, format!("Write Error: {e}")),
                    }
                },
            }
        ),
        Err(e) => Err(
            match stream.write_all(GatewayTimeout.as_bytes()).await {
                Ok(_) => io::Error::new(io::ErrorKind::TimedOut, format!("Connection to upstream timed out: {e}")),
                Err(e) => io::Error::new(io::ErrorKind::InvalidData, format!("Write Error: {e}")),
            }),
    }
}