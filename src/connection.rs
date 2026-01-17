use crate::config::Config;
use crate::http;
use crate::http::StatusCode::{BadGateway, BadRequest, Forbidden, GatewayTimeout, NotFound, RequestTimeout, Unauthorized};
use crate::http::{HeaderKey, StatusCode};
use io::ErrorKind;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{copy_bidirectional, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info, instrument, warn};

async fn handle_error(stream: &mut TcpStream, status_code: StatusCode, e: impl Into<String>) -> io::Error {
    if let Err(e) = stream.write_all(status_code.as_bytes()).await {
        error!("Stream write error: {}", e);
        return e;
    }

    let err = e.into();
    debug!("{}", err);
    io::Error::new(ErrorKind::InvalidData, err)
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

    match stream.peer_addr() {
        Ok(socket_addr) => {
            let ip = socket_addr.ip();
            if !config.permit_addr(ip) {
                warn!("Access denied for peer {}", ip);
                Err(handle_error(&mut stream, Forbidden, "Access denied").await)
            } else {
                Ok(())
            }
        },
        Err(e) => {
            Err(handle_error(&mut stream, BadRequest, e.to_string()).await)
        }
    }?;

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
        None => Err(handle_error(&mut stream, BadRequest, "Parsing headers failed").await),
    }?;

    let auth_result = request.headers.get(&HeaderKey(b"authorization")).map(|auth| config.permit_user(auth));
    let api_result = request.headers.get(&HeaderKey(b"x-api-key")).map(|key| config.permit_api_key(key));

    let _ = match (auth_result, api_result) {
        (Some(false), _) => Err(handle_error(&mut stream, Unauthorized(true), "Invalid Basic Authentication").await),
        (_, Some(false)) => Err(handle_error(&mut stream, Unauthorized(false), "Invalid API Key").await),
        (None, None) if config.is_proxy_private() => Err(handle_error(&mut stream, Unauthorized(false), "No authorization method used").await),
        _ => Ok(())
    }?;

    let route = match config.select_upstream(&request) {
        Some(a) => Ok(a),
        None => Err(handle_error(&mut stream, NotFound, "Failed to select upstream server").await),
    }?;

    let timeout = Duration::from_millis(route.timeout.unwrap_or(10_000));

    let backend = match route.next_addr().await {
        Some(b) => b,
        None => return Err(handle_error(&mut stream, BadGateway, "Failed to fetch next backend").await),
    };

    let result = tokio::time::timeout(timeout, async {
        let mut upstream = TcpStream::connect(&backend).await?;
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