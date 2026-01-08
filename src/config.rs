use crate::http::{HeaderKey, Request};
use crate::routing::Route;
use serde::Deserialize;
use std::fs::read_to_string;
use std::io;
use std::io::ErrorKind::InvalidData;
use std::io::Error;
use std::path::Path;

#[derive(Deserialize)]
pub struct Config {
    routes: Vec<Route>
}

impl Config {
    pub fn new(config: &String) -> io::Result<Self> {
        match Path::new(&config).extension().map(|ext| ext.to_ascii_lowercase()) {
            Some(ext) => {
                let data = read_to_string(&config).map_err(|e| Error::new(InvalidData, format!("Failed to load config file {config}: {e}")))?;
                match ext.to_ascii_lowercase().to_str() {
                    Some("toml") => toml::from_str(&data).map_err(|e| Error::new(InvalidData, e)),
                    Some("yaml") | Some("yml") => serde_yaml::from_str(&data).map_err(|e| Error::new(InvalidData, e)),
                    _ => Err(Error::new(InvalidData, format!("Unsupported config extension {:?}", ext))),
                }
            },
            _ => Err(Error::new(InvalidData, "Config extension missing [use .toml, .yaml or .yml"))
        }
    }

    pub fn add_routes(&mut self, routes: Vec<Route>) {
        self.routes.extend(routes);
    }

    pub fn select_upstream(&self, request: &Request) -> Option<&str> {
        self.routes.iter().find(|r| {
            request.headers.get(&HeaderKey("host".as_bytes())).map(|h| *h == r.host).unwrap_or(false) &&
                request.path.starts_with(&r.path)
        }).map(|r| r.addr.as_str())
    }

}