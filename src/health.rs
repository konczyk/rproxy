use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{error, info, trace};

pub async fn check(path: String, backends: Vec<String>, active: Arc<RwLock<Vec<String>>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        let mut healthy = Vec::with_capacity(backends.len());
        for backend in &backends {
            if let Ok(Ok(_)) = tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(&backend)).await {
                healthy.push(backend.clone());
            }
        }

        let mut current = active.write().await;
        if healthy.is_empty() {
            error!("All backends for route {} are DOWN!", path);
        } else if current.is_empty() && !healthy.is_empty() {
            info!("Route {} has {} healthy backends again", path, healthy.len());
        } else {
            trace!("Route {} has {} healthy backends", path, healthy.len());
        }

        *current = healthy;

    }
}