use rproxy::connection;
use rproxy::routing::Route;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

mod tests {
    use super::*;
    use rproxy::config::{AccessControl, Auth, Config};
    use std::collections::{HashMap, HashSet};
    use std::net::IpAddr;
    use std::str::FromStr;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;
    use base64::Engine;
    use base64::prelude::BASE64_STANDARD;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::sleep;

    async fn connect(routes: Vec<Route>, access: Option<AccessControl>) -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let c = "tests/config.toml".to_string();

        let mut c = Config::new(&c).unwrap();
        c.add_routes(routes);
        c.access = access;
        let config = Arc::new(c);

        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = connection::handle_connection(stream, config).await;
            }
        });

        TcpStream::connect(addr).await.unwrap()
    }

    #[tokio::test]
    async fn test_200_on_successful_proxy_to_backend() {
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

        let routes = vec![Route {
            host: "localhost:8080".as_bytes().to_vec(),
            path: "/target".as_bytes().to_vec(),
            backends: vec![backend_addr.to_string()],
            timeout: None,
            counter: AtomicUsize::new(0)
        }];
        let mut client = connect(routes, None).await;

        client.write_all(b"GET /target HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("Hello"));
    }

    #[tokio::test]
    async fn test_200_on_successful_proxy_to_backend_with_api_key() {
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

        let routes = vec![Route {
            host: "localhost:8080".as_bytes().to_vec(),
            path: "/target".as_bytes().to_vec(),
            backends: vec![backend_addr.to_string()],
            timeout: None,
            counter: AtomicUsize::new(0)
        }];

        let access = AccessControl {
            whitelist: None,
            auth: Some(Auth {
                users: None,
                api_keys: Some(HashSet::from(["api_key1".to_string()])),
            }),
        };

        let mut client = connect(routes, Some(access)).await;
        client.write_all(b"GET /target HTTP/1.1\r\nHost: localhost:8080\r\nX-Api-Key: api_key1\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("Hello"));
    }

    #[tokio::test]
    async fn test_200_on_successful_proxy_to_backend_with_basic_auth() {
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

        let routes = vec![Route {
            host: "localhost:8080".as_bytes().to_vec(),
            path: "/target".as_bytes().to_vec(),
            backends: vec![backend_addr.to_string()],
            timeout: None,
            counter: AtomicUsize::new(0)
        }];

        let encoded = BASE64_STANDARD.encode("admin:password");
        let access = AccessControl {
            whitelist: None,
            auth: Some(Auth {
                users: Some(HashMap::from([("admin".to_string(), "password".to_string())])),
                api_keys: None,
            }),
        };

        let mut client = connect(routes, Some(access)).await;
        client.write_all(format!("GET /target HTTP/1.1\r\nHost: localhost:8080\r\nAuthorization: Basic {encoded}\r\n\r\n").as_bytes()).await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("Hello"));
    }

    #[tokio::test]
    async fn test_400_on_missing_headers_delimiter() {
        let mut client = connect(vec![], None).await;
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("400 Bad Request"));
    }

    #[tokio::test]
    async fn test_400_on_malformed_request_line() {
        let mut client = connect(vec![], None).await;
        client.write_all(b"xxx\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("400 Bad Request"));
    }

    #[tokio::test]
    async fn test_401_on_invalid_basic_auth() {
        let encoded = BASE64_STANDARD.encode("admin:password1");
        let access = AccessControl {
            whitelist: None,
            auth: Some(Auth {
                users: Some(HashMap::from([("admin".to_string(), "password".to_string())])),
                api_keys: None,
            }),
        };
        let mut client = connect(vec![], Some(access)).await;
        client.write_all(format!("GET / HTTP/1.1\r\nHost: localhost:8080\r\nAuthorization: Basic {encoded}\r\n\r\n").as_bytes()).await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = [0u8; 1024];
        client.read(&mut response).await.unwrap();

        let res = str::from_utf8(&response).unwrap();
        assert!(res.contains("401 Unauthorized"));
        assert!(res.contains("WWW-Authenticate: Basic realm=\"Proxy\""));
    }

    #[tokio::test]
    async fn test_401_on_invalid_api_key() {
        let access = AccessControl {
            whitelist: None,
            auth: Some(Auth {
                users: None,
                api_keys: Some(HashSet::from(["api_key1".to_string()])),
            }),
        };
        let mut client = connect(vec![], Some(access)).await;
        client.write_all(b"GET / HTTP/1.1\r\nHost: localhost:8080\r\nX-Api-Key: api_key2\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = [0u8; 1024];
        client.read(&mut response).await.unwrap();

        let res = str::from_utf8(&response).unwrap();
        assert!(res.contains("401 Unauthorized"));
        assert!(!res.contains("WWW-Authenticate: Basic realm=\"Proxy\""));
    }

    #[tokio::test]
    async fn test_403_on_non_whitelisted_ip() {
        let access = AccessControl {
            whitelist: Some(HashSet::from([IpAddr::from_str("1.1.1.1").unwrap()])),
            auth: None,
        };
        let mut client = connect(vec![], Some(access)).await;
        client.write_all(b"GET / HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = [0u8; 1024];
        client.read(&mut response).await.unwrap();

        assert!(str::from_utf8(&response).unwrap().contains("403 Forbidden"));
    }

    #[tokio::test]
    async fn test_404_on_missing_route() {
        let mut client = connect(vec![], None).await;
        client.write_all(b"GET /buba HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("404 Not Found"));
    }

    #[tokio::test]
    async fn test_502_on_invalid_gateway() {
        let routes = vec![Route {
            host: "localhost:54322".as_bytes().to_vec(),
            path: "/target".as_bytes().to_vec(),
            backends: vec!["localhost:54321".to_string()],
            timeout: None,
            counter: AtomicUsize::new(0)
        }];
        let mut client = connect(routes, None).await;
        client.write_all(b"GET /target HTTP/1.1\r\nHost: localhost:54322\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("502 Bad Gateway"));
    }

    #[tokio::test]
    async fn test_504_on_upstream_timeout() {
        let backend_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_addr = backend_listener.local_addr().unwrap();

        tokio::spawn(async move {
            sleep(Duration::from_millis(1000)).await;
            if let Ok((mut stream, _)) = backend_listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;
                let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
                let _ = stream.write_all(response).await;
            }
        });

        let routes = vec![Route {
            host: "localhost:8080".as_bytes().to_vec(),
            path: "/target".as_bytes().to_vec(),
            backends: vec![backend_addr.to_string()],
            timeout: Some(100),
            counter: AtomicUsize::new(0)
        }];
        let mut client = connect(routes, None).await;

        client.write_all(b"GET /target HTTP/1.1\r\nHost: localhost:8080\r\n\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("504 Gateway Timeout"));
    }

}