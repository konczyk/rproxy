use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tracing::{debug, error, warn};

#[derive(Error, Debug)]
pub enum HeadersParseError {
    #[error("Header Fields Too Large")]
    TooLarge,
    #[error("Bad Request")]
    Invalid
}

impl HeadersParseError {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            HeadersParseError::TooLarge => StatusCode::HeadersTooLarge,
            HeadersParseError::Invalid => StatusCode::BadRequest
        }
    }
}

pub enum StatusCode {
    BadRequest,
    Forbidden,
    NotFound,
    RequestTimeout,
    HeadersTooLarge,
    BadGateway,
    GatewayTimeout,
}

impl StatusCode {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::BadRequest => b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::Forbidden => b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::NotFound => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::RequestTimeout => b"HTTP/1.1 408 Request Timeout\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::HeadersTooLarge => b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::BadGateway => b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::GatewayTimeout => b"HTTP/1.1 504 Gateway Timeout\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        }
    }
}

#[derive(Debug)]
pub struct HeaderKey<'a>(pub &'a [u8]);

impl<'a> PartialEq for HeaderKey<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

impl<'a> Eq for HeaderKey<'a> {}

impl<'a> Hash for HeaderKey<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for &b in self.0 {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

#[derive(Debug)]
pub struct Request<'a> {
    pub method: &'a [u8],
    pub path: &'a [u8],
    pub headers: HashMap::<HeaderKey<'a>, &'a [u8]>,
}

impl<'a> Request<'a> {
    pub fn new(buf: &'a [u8]) -> Option<Request<'a>> {
        let mut lines = buf.split(|x| *x == b'\n').map(|x| x.trim_ascii_end());

        if let Some((method, path)) = lines.next().and_then(|line| {
            let mut req = line.split(|x| *x == b' ').into_iter();
            let method = req.next();
            let path = req.next();
            method.and_then(|m| path.map(|p| (m, p)))
        }) {
            let mut headers = HashMap::new();
            lines.skip_while(|x| x.is_empty()).take_while(|x| !x.is_empty()).for_each(|line| {
                let mut h = line.splitn(2, |x| *x == b':').map(|x| x.trim_ascii()).into_iter();
                let header = h.next();
                let value = h.next();
                match header.and_then(|h| value.map(|v| (h, v))) {
                    Some((h, v)) => {
                        headers.insert(HeaderKey(h), v);
                    },
                    None => {
                        warn!("Failed parsing header: {:?}", line)
                    },
                }
            });

            return Some(Request { method, path, headers })
        }

        None
    }

    pub async fn parse_headers(stream: &mut TcpStream) -> Result<(Vec<u8>, usize), HeadersParseError> {
        let mut buf = [0u8; 512];
        let mut headers = Vec::with_capacity(2048);
        debug!("Parsing request headers");

        loop {
            match stream.read(&mut buf).await {
                Ok(bytes) => {
                    if bytes == 0 {
                        warn!("Reading headers returned 0 bytes");
                        return Err(HeadersParseError::Invalid)
                    }
                    headers.extend_from_slice(&buf[..bytes]);
                    if headers.len() >= 64 * 1024 {
                        warn!("Requested headers too large: {}", headers.len());
                        return Err(HeadersParseError::TooLarge);
                    }
                    match headers.windows(4).enumerate().find(|(_, val)| val == b"\r\n\r\n") {
                        Some((i, _)) => {
                            return Ok((headers, i + 4));
                        },
                        _ => ()
                    }
                },
                Err(e) => {
                    error!("Failed to read headers: {}", e);
                    return Err(HeadersParseError::Invalid)
                }
            }
        }
    }
}