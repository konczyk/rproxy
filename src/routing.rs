use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;
use tracing::{error, trace};

#[derive(Deserialize)]
pub struct RouteConfig {
    pub host: String,
    pub path: String,
    pub backends: Vec<String>,
    pub timeout: Option<u64>,
}

#[derive(Deserialize)]
#[serde(from = "RouteConfig")]
pub struct Route {
    pub host: Vec<u8>,
    pub path: Vec<u8>,
    pub backends: Vec<String>,
    pub timeout: Option<u64>,
    #[serde(skip)]
    pub counter: AtomicUsize,
    #[serde(skip)]
    pub active_backends: Arc<RwLock<Vec<String>>>,
}

impl Route {
    pub async fn next_addr(&self) -> Option<String> {
        if self.backends.len() == 0 {
            error!(
                "No backends configured for route: {}",
                str::from_utf8(self.path.as_slice()).unwrap_or("")
            );
            return None;
        }

        let active = self.active_backends.read().await;
        if active.len() == 0 {
            error!(
                "No active backends found for route: {}",
                str::from_utf8(self.path.as_slice()).unwrap_or("")
            );
            return None;
        }

        let idx = self.counter.fetch_add(1, Ordering::SeqCst) % active.len();
        let backend = &active[idx];
        trace!(
            "Selected backend {} from route {}",
            backend,
            str::from_utf8(self.path.as_slice()).unwrap_or("")
        );
        Some(backend.clone())
    }
}

impl From<RouteConfig> for Route {
    fn from(value: RouteConfig) -> Self {
        Self {
            host: value.host.as_bytes().to_vec(),
            path: value.path.as_bytes().to_vec(),
            backends: value.backends.clone(),
            timeout: value.timeout,
            counter: AtomicUsize::new(0),
            active_backends: Arc::new(RwLock::new(value.backends)),
        }
    }
}
