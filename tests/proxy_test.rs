use rproxy::routing::Routing;
use rproxy::connection;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

mod tests {
    use super::*;
    use rproxy::routing::Route;

    fn connect(routing: Option<Routing>) -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let routing = Arc::new(routing.unwrap_or(Routing::new(vec![])));

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
            delay *= 2;
        }

        TcpStream::connect(addr).unwrap()
    }

    #[test]
    fn test_400_on_missing_headers_delimiter() {
        let mut client = connect(None);
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(response.contains("400 Bad Request"));
    }

    #[test]
    fn test_404_on_missing_route() {
        let mut client = connect(None);
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(response.contains("404 Not Found"));
    }

    #[test]
    fn test_400_on_malformed_request_line() {
        let mut client = connect(None);
        client.write_all(b"xxx\r\n\r\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(response.contains("400 Bad Request"));
    }

    #[test]
    fn test_successful_proxy_to_backend() {
        let backend_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend_addr = backend_listener.local_addr().unwrap();

       thread::spawn(move || {
            if let Ok((mut stream, _)) = backend_listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
                let _ = stream.write_all(response);
            }
        });

        let routes = vec![
            Route { host: "localhost:8080".as_bytes().to_vec(), path: "/target".as_bytes().to_vec(), addr: backend_addr.to_string() }
        ];
        let mut client = connect(Some(Routing::new(routes)));
        client.write_all(b"GET /target HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("Hello"));

    }
}