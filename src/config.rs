use crate::http::{HeaderKey, Request};
use crate::routing::Route;

pub struct Config {
    routes: Vec<Route>
}

impl Config {
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    pub fn select_upstream(&self, request: &Request) -> Option<&str> {
        self.routes.iter().find(|r| {
            request.headers.get(&HeaderKey("host".as_bytes())).map(|h| *h == r.host).unwrap_or(false) &&
                request.path.starts_with(&r.path)
        }).map(|r| r.addr.as_str())
    }

}