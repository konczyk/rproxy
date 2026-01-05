use crate::http::{HeaderKey, Request};

struct Route {
    host: Vec<u8>,
    path: Vec<u8>,
    addr: String,
}

pub struct Routing {
    routes: Vec<Route>
}

impl Routing {
    pub fn new() -> Routing {
        Routing {
            routes: vec![
                Route { host: "localhost:8080".as_bytes().to_vec(), path: "/pl".as_bytes().to_vec(), addr: "192.168.124.185:80".to_string() }
            ]
        }
    }
    pub fn select_upstream(&self, request: &Request) -> Option<&str> {
        self.routes.iter().find(|r| {
            request.headers.get(&HeaderKey("host".as_bytes())).map(|h| *h == r.host).unwrap_or(false) &&
                request.path.starts_with(&r.path)
        }).map(|r| r.addr.as_str())
    }

}