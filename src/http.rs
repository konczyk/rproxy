use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::net::TcpStream;

pub enum HeadersParseError {
    TooLarge,
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
    NotFound,
    HeadersTooLarge,
    BadGateway,
}

impl StatusCode {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::BadRequest => b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::NotFound => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::HeadersTooLarge => b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Self::BadGateway => b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
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
                        eprintln!("Failed parsing header: {:?}", line)
                    },
                }
            });

            return Some(Request { method, path, headers })
        }

        None
    }

    pub fn parse_headers(stream: &mut TcpStream) -> Result<(Vec<u8>, usize), HeadersParseError> {
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).map_err(|_| HeadersParseError::Invalid)?;

        let mut buf = [0u8; 512];
        let mut headers = Vec::with_capacity(2048);

        loop {
            match stream.read(&mut buf) {
                Ok(bytes) if bytes > 0 => {
                    headers.extend_from_slice(&buf[..bytes]);
                    if headers.len() >= 64 * 1024 {
                        return Err(HeadersParseError::TooLarge);
                    }
                    match headers.windows(4).enumerate().find(|(_, val)| val == b"\r\n\r\n") {
                        Some((i, _)) => {
                            return Ok((headers, i + 4));
                        },
                        _ => ()
                    }
                },
                _ => return Err(HeadersParseError::Invalid)
            }
        }
    }
}