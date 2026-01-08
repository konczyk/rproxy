use serde::Deserialize;

#[derive(Deserialize)]
pub struct RouteConfig {
    pub host: String,
    pub path: String,
    pub addr: String,
}

#[derive(Deserialize)]
#[serde(from="RouteConfig")]
pub struct Route {
    pub host: Vec<u8>,
    pub path: Vec<u8>,
    pub addr: String,
}

impl From<RouteConfig> for Route {
    fn from(value: RouteConfig) -> Self {
        Self {
            host: value.host.as_bytes().to_vec(),
            path: value.path.as_bytes().to_vec(),
            addr: value.addr
        }
    }
}