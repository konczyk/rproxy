use crate::http::{HeaderKey, Request};

pub struct Route {
    pub host: Vec<u8>,
    pub path: Vec<u8>,
    pub addr: String,
}

pub struct Routing {
    routes: Vec<Route>
}

impl Routing {
    pub fn new(routes: Vec<Route>) -> Routing {
        Routing { routes }
    }

    pub fn select_upstream(&self, request: &Request) -> Option<&str> {
        self.routes.iter().find(|r| {
            request.headers.get(&HeaderKey("host".as_bytes())).map(|h| *h == r.host).unwrap_or(false) &&
                request.path.starts_with(&r.path)
        }).map(|r| r.addr.as_str())
    }

}