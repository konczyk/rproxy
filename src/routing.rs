use crate::http::{HeaderKey, Request};

struct Route {
    host: String,
    path: String,
    addr: String,
}

pub struct Routing {
    routes: Vec<Route>
}

impl Routing {
    pub fn new() -> Routing {
        Routing {
            routes: vec![
                Route { host: "localhost:8080".to_string(), path: "/pl".to_string(), addr: "192.168.124.185:80".to_string() }
            ]
        }
    }
    pub fn select_upstream(&self, request: &Request) -> Option<String> {
        self.routes.iter().find(|r| {
            request.headers.get(&HeaderKey("host".as_bytes())).map(|h| *h == r.host.as_bytes()).unwrap_or(false) &&
                r.path.to_lowercase().as_bytes().starts_with(request.path)
        }).map(|r| r.addr.clone())
    }

}