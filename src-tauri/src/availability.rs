//! Conservative, live admission check. Indexer seed counts never admit a result.
use crate::{dht, model::Torrent};
use futures_util::{stream::FuturesUnordered, StreamExt};
use mainline::async_dht::AsyncDht;
use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddr,
    time::Duration,
};

#[derive(Default)]
pub struct Verifier {
    node: tokio::sync::OnceCell<AsyncDht>,
    runtime: Option<std::path::PathBuf>,
}
impl Verifier {
    pub fn new(runtime: std::path::PathBuf) -> Self {
        Self {
            runtime: Some(runtime),
            ..Default::default()
        }
    }
    async fn node(&self) -> Result<&AsyncDht, String> {
        self.node
            .get_or_try_init(|| async {
                mainline::Dht::builder()
                    .port(0)
                    .request_timeout(Duration::from_secs(3))
                    .build()
                    .map(|d| d.as_async())
                    .map_err(|e| e.to_string())
            })
            .await
    }
    pub async fn check(&self, item: Torrent) -> Result<Torrent, String> {
        if item.network == "ed2k" {
            // Server evidence has a different label; never call it a direct peer check.
            let recent = item
                .availability_checked_at
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .is_some_and(|t| {
                    (0..60).contains(&(chrono::Utc::now().timestamp() - t.timestamp()))
                });
            if recent
                && item.availability_status == "reported_complete"
                && item.complete_sources_reported.unwrap_or(0) > 0
            {
                return Ok(item);
            }
            return Err("Sin fuentes completas eD2k reportadas recientemente".into());
        }
        if crate::v2::pure(&item.magnet_uri) {
            let runtime = self.runtime.clone().ok_or("Motor v2 no disponible")?;
            return tokio::task::spawn_blocking(move || crate::v2::verify(&runtime, item))
                .await
                .map_err(|e| e.to_string())?;
        }
        tokio::time::timeout(Duration::from_secs(25), self.check_inner(item))
            .await
            .map_err(|_| "No se encontró una fuente completa accesible a tiempo".to_string())?
    }
    async fn check_inner(&self, mut item: Torrent) -> Result<Torrent, String> {
        let hash: [u8; 20] = hex::decode(&item.infohash_v1)
            .map_err(|_| "Hash inválido")?
            .try_into()
            .map_err(|_| "Hash v1 inválido")?;
        let node = self.node().await?;
        let mut peers = node.get_peers(mainline::Id::from_bytes(hash).map_err(|e| e.to_string())?);
        let mut jobs = FuturesUnordered::new();
        let mut seen = HashSet::new();
        let mut queued = VecDeque::new();
        let mut exhausted = false;
        loop {
            while jobs.len() < 4 {
                let Some(peer) = queued.pop_front() else {
                    break;
                };
                jobs.push(async move {
                    tokio::time::timeout(
                        Duration::from_secs(8),
                        dht::inspect(SocketAddr::V4(peer), hash, true),
                    )
                    .await
                });
            }
            if exhausted && jobs.is_empty() {
                return Err("Sin fuente completa accesible".into());
            }
            tokio::select! {
                batch = peers.next(), if !exhausted => {
                    match batch {
                        Some(batch) => {
                            for peer in batch.into_iter().filter(|a| dht::public(*a)) {
                                if seen.len() >= 12 { exhausted = true; break; }
                                if seen.insert(peer) {
                                    queued.push_back(peer);
                                }
                            }
                        }
                        None => exhausted = true,
                    }
                }
                result = jobs.next(), if !jobs.is_empty() => {
                    if let Some(Ok(Ok(observed))) = result {
                        // Identity, name, size and files come from info-hash verified metadata.
                        item.title = observed.title;
                        item.size_bytes = observed.size_bytes;
                        item.files = observed.files;
                        item.availability_status = observed.availability_status;
                        item.availability_checked_at = observed.availability_checked_at;
                        return Ok(item);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Comprueba disponibilidad real del torrent público de Ubuntu"]
    async fn live_availability() {
        let client = crate::sources::client().unwrap();
        let rows = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        let output = rows.clone();
        let source = crate::sources::registry()
            .into_iter()
            .find(|s| s.id() == "ubuntu")
            .unwrap();
        source
            .search(
                &client,
                "ubuntu",
                std::sync::Arc::new(move |b| output.lock().unwrap().extend(b)),
            )
            .await
            .unwrap();
        let candidates = rows.lock().unwrap().clone();
        let verifier = Verifier::default();
        let mut successful = 0;
        for item in candidates.into_iter().take(3) {
            let name = item.title.clone();
            let result = verifier.check(item).await;
            println!(
                "{name}: {:?}",
                result
                    .as_ref()
                    .map(|t| (&t.availability_status, &t.availability_checked_at))
            );
            if result.is_ok() {
                successful += 1;
                break;
            }
        }
        assert!(
            successful > 0,
            "No se pudo comprobar una fuente completa desde esta red"
        );
    }
}
