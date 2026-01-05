use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use rproxy::{connection, routing};

mod tests {
    use super::*;

    fn connect() -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let routing = Arc::new(routing::Routing::new());

        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let _ = connection::handle_connection(stream, routing);
        });

        let mut delay = std::time::Duration::from_millis(10);
        for _ in 0..5 {
            if let Ok(stream) = TcpStream::connect(addr) {
                return stream
            }
            thread::sleep(delay);
            delay *= 2; // Exponential backoff
        }

        TcpStream::connect(addr).unwrap()
    }

    #[test]
    fn test_400_on_missing_headers_delimiter() {
        let mut client = connect();
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(response.contains("400 Bad Request"));
    }

    #[test]
    fn test_404_on_missing_route() {
        let mut client = connect();
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(response.contains("404 Not Found"));
    }

    #[test]
    fn test_400_on_malformed_request_line() {
        let mut client = connect();
        client.write_all(b"xxx\r\n\r\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(response.contains("400 Bad Request"));
    }
}