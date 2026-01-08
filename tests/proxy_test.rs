use rproxy::connection;
use rproxy::routing::Route;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

mod tests {
    use super::*;
    use rproxy::config::Config;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn connect(routing: Option<Config>) -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let routing = Arc::new(routing.unwrap_or(Config::new(vec![])));

        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = connection::handle_connection(stream, routing).await;
            }
        });

        TcpStream::connect(addr).await.unwrap()
    }

    #[tokio::test]
    async fn test_400_on_missing_headers_delimiter() {
        let mut client = connect(None).await;
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("400 Bad Request"));
    }

    #[tokio::test]
    async fn test_404_on_missing_route() {
        let mut client = connect(None).await;
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("404 Not Found"));
    }

    #[tokio::test]
    async fn test_400_on_malformed_request_line() {
        let mut client = connect(None).await;
        client.write_all(b"xxx\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("400 Bad Request"));
    }

    #[tokio::test]
    async fn test_successful_proxy_to_backend() {
        let backend_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_addr = backend_listener.local_addr().unwrap();

       tokio::spawn(async move {
            if let Ok((mut stream, _)) = backend_listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
                let _ = stream.write_all(response).await;
            }
        });

        let routes = vec![
            Route { host: "localhost:8080".as_bytes().to_vec(), path: "/target".as_bytes().to_vec(), addr: backend_addr.to_string() }
        ];
        let mut client = connect(Some(Config::new(routes))).await;
        client.write_all(b"GET /target HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("Hello"));

    }
}