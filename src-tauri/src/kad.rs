//! Kad search through the same private aMuled engine used for eD2k transfers.
use crate::{
    ec::Tag,
    ed2k_downloads::Downloads,
    model::Torrent,
    sources::{Batch, TorrentSource},
};
use std::{path::Path, sync::Arc, time::Duration};
pub struct KadSource {
    pub engine: Arc<Downloads>,
}
impl TorrentSource for KadSource {
    fn id(&self) -> &str {
        "kad"
    }
    fn search<'a>(
        &'a self,
        _: &'a reqwest::Client,
        query: &'a str,
        batch: Batch,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>>
    {
        Box::pin(async move { self.engine.search_kad(query, batch).await })
    }
}
pub fn ready(data: &Path) -> bool {
    let path = data.join("nodes.dat");
    path.metadata().is_ok_and(|m| m.len() <= 1024 * 1024)
        && std::fs::read(path).is_ok_and(|bytes| validate_nodes(&bytes).is_ok())
}
pub async fn bootstrap(data: &Path) -> Result<(), String> {
    let path = data.join("nodes.dat");
    if ready(data) {
        return Ok(());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get("https://upd.emule-security.org/nodes.dat")
        .send()
        .await
        .map_err(|e| format!("No se pudo iniciar Kad: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut bytes = vec![];
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > 1024 * 1024 {
            return Err("Lista Kad demasiado grande".into());
        }
        bytes.extend(chunk)
    }
    validate_nodes(&bytes)?;
    let temp = data.join("nodes.dat.new");
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(temp, path).map_err(|e| e.to_string())
}
fn validate_nodes(bytes: &[u8]) -> Result<(), String> {
    let word = |i| {
        bytes
            .get(i..i + 4)
            .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
    };
    if word(0) != Some(0) {
        return Err("Cabecera Kad inválida".into());
    }
    let (offset, stride) = match word(4) {
        Some(1) => (12, 25),
        Some(2) => (12, 34),
        Some(3) if word(8) == Some(1) => (16, 25),
        _ => return Err("Versión de lista Kad no admitida".into()),
    };
    let count = word(offset - 4).ok_or("Lista Kad truncada")? as usize;
    if count == 0 || count > 10000 || bytes.len() != offset + count * stride {
        return Err("Lista Kad incompleta o vacía".into());
    }
    Ok(())
}
pub fn results(tags: &[Tag]) -> Vec<Torrent> {
    tags.iter()
        .filter(|t| t.id == 0x700)
        .filter_map(|r| {
            let hash = r.get(0x31e)?;
            if hash.data.len() != 16 {
                return None;
            }
            let size = r.value(0x303);
            let complete = r.value(0x30d);
            let name = r.get(0x301)?.text();
            if size == 0
                || complete == 0
                || name.is_empty()
                || name.len() > 4096
                || name.chars().any(char::is_control)
            {
                return None;
            }
            let hash = hex::encode(&hash.data);
            let now = chrono::Utc::now().to_rfc3339();
            Some(Torrent {
                id: format!("ed2k:{hash}:{size}"),
                title: name.clone(),
                infohash_v1: String::new(),
                magnet_uri: String::new(),
                size_bytes: Some(size),
                published_at: None,
                seeds_reported: None,
                availability_status: "reported_complete".into(),
                availability_checked_at: Some(now.clone()),
                sources: vec!["kad".into()],
                files: vec![],
                content_types: vec![],
                network: "ed2k".into(),
                ed2k_uri: Some(crate::ed2k::link(&name, size, &hash)),
                complete_sources_reported: Some(complete),
                discovered_at: now,
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_bootstrap_and_evidence() {
        let mut bytes = vec![0; 12];
        bytes[4] = 2;
        bytes[8] = 1;
        bytes.extend([0; 34]);
        assert!(validate_nodes(&bytes).is_ok());
        bytes.pop();
        assert!(validate_nodes(&bytes).is_err());
        let row = Tag {
            id: 0x700,
            kind: 4,
            data: vec![1],
            children: vec![
                Tag::hash(0x31e, vec![7; 16]),
                Tag::string(0x301, "manual.pdf"),
                Tag::number(0x303, 100),
                Tag::number(0x30d, 1),
            ],
        };
        let parsed = results(&[row.clone()]);
        assert_eq!(parsed[0].sources, vec!["kad"]);
        assert_eq!(parsed[0].availability_status, "reported_complete");
        let mut incomplete = row;
        incomplete.children.retain(|t| t.id != 0x30d);
        assert!(results(&[incomplete]).is_empty());
    }
}
