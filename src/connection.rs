use crate::config::Config;
use crate::http;
use crate::http::StatusCode;
use crate::http::StatusCode::{BadGateway, BadRequest, GatewayTimeout, NotFound, RequestTimeout};
use io::ErrorKind;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{copy_bidirectional, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{error, info, instrument, warn};

async fn handle_error(stream: &mut TcpStream, status_code: StatusCode, e: impl Into<String>) -> io::Error {
    if let Err(e) = stream.write_all(status_code.as_bytes()).await {
        error!("Stream write error: {}", e);
        return e;
    }

    io::Error::new(ErrorKind::InvalidData, e.into())
}

#[instrument(
    name="proxy-request",
    skip(stream,config),
    fields(
        request_id = tracing::field::Empty,
        client = %stream.peer_addr().unwrap_or("0.0.0.0:0".parse().unwrap())
    )
)]
pub async fn handle_connection(mut stream: TcpStream, config: Arc<Config>) -> io::Result<()> {
    let request_id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("request_id", &request_id);

    info!("Processing new request");

    let (headers, end) = match tokio::time::timeout(Duration::from_secs(5), http::Request::parse_headers(&mut stream)).await {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(e)) => Err(handle_error(&mut stream, e.to_status_code(), e.to_string()).await),
        Err(e) => {
            warn!("Client header timeout: {}", e);
            Err(handle_error(&mut stream, RequestTimeout, e.to_string()).await)
        },
    }?;

    let request = match http::Request::new(&headers[..end]) {
        Some(req) => Ok(req),
        None => Err(handle_error(&mut stream, BadRequest, "Unspecified error").await),
    }?;

    let route = match config.select_upstream(&request) {
        Some(a) => Ok(a),
        None => Err(handle_error(&mut stream, NotFound, "Unspecified Error").await),
    }?;

    let timeout = Duration::from_millis(route.timeout.unwrap_or(10_000));

    let result = tokio::time::timeout(timeout, async {
        let mut upstream = TcpStream::connect(&route.addr).await?;
        upstream.write_all(&headers[..end-2]).await?;
        upstream.write_all(format!("X-Request-ID: {}\r\n\r\n", request_id).as_bytes()).await?;

        copy_bidirectional(&mut stream, &mut upstream).await
    }).await;

    match result {
        Ok(Ok(_)) => {
            Ok(())
        },
        Ok(Err(e)) => Err(
            match e.kind() {
                ErrorKind::ConnectionRefused | ErrorKind::AddrNotAvailable | ErrorKind::Other => handle_error(&mut stream, BadGateway, e.to_string()).await,
                ErrorKind::TimedOut => handle_error(&mut stream, GatewayTimeout, e.to_string()).await,
                _ => handle_error(&mut stream, BadRequest, e.to_string()).await
            }
        ),
        Err(e) => Err({
            error!("Connection to upstream timed out: {}", e);
            handle_error(&mut stream, GatewayTimeout, e.to_string()).await
        }),
    }
}