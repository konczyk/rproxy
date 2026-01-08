use crate::config::Config;
use crate::http;
use crate::http::StatusCode::{BadRequest, NotFound};
use std::io;
use std::sync::Arc;
use tokio::io::{copy_bidirectional, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_connection(mut stream: TcpStream, config: Arc<Config>) -> io::Result<()> {

    let (headers, end) = match http::Request::parse_headers(&mut stream).await {
        Ok(val) => Ok(val),
        Err(e) => Err(
            match stream.write_all(e.to_status_code().as_bytes()).await {
                Ok(_) => io::Error::new(io::ErrorKind::InvalidData, "Protocol Error"),
                Err(e) => io::Error::new(io::ErrorKind::InvalidData, format!("Write Error: {e}")),
            }),
    }?;

    let request = match http::Request::new(&headers[..end]) {
        Some(req) => Ok(req),
        None => Err(
            match stream.write_all(BadRequest.as_bytes()).await {
                Ok(_) => io::Error::new(io::ErrorKind::InvalidData, "Request Error"),
                Err(e) => io::Error::new(io::ErrorKind::InvalidData, format!("Write Error: {e}")),
            }),
    }?;

    let addr = match config.select_upstream(&request) {
        Some(a) => Ok(a),
        None => Err(
            match stream.write_all(NotFound.as_bytes()).await {
                Ok(_) => io::Error::new(io::ErrorKind::InvalidData, "Routing Error"),
                Err(e) => io::Error::new(io::ErrorKind::InvalidData, format!("Write Error: {e}")),
            }),
    }?;

    let mut upstream = TcpStream::connect(addr).await?;
    upstream.write_all(&headers).await?;

    match copy_bidirectional(&mut stream, &mut upstream).await {
        Ok(_) => Ok(()),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("Tunner Error: {e}")))
    }?;

    Ok(())
}