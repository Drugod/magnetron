use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub id: String,
    pub title: String,
    pub infohash_v1: String,
    pub magnet_uri: String,
    pub size_bytes: Option<u64>,
    pub published_at: Option<String>,
    pub seeds_reported: Option<u64>,
    pub availability_status: String,
    pub availability_checked_at: Option<String>,
    pub sources: Vec<String>,
    pub files: Vec<String>,
    #[serde(default)]
    pub content_types: Vec<String>,
    #[serde(default = "bittorrent")]
    pub network: String,
    #[serde(default)]
    pub ed2k_uri: Option<String>,
    #[serde(default)]
    pub complete_sources_reported: Option<u64>,
    pub discovered_at: String,
}
fn bittorrent() -> String {
    "bittorrent".into()
}

pub fn validate_magnet(value: &str) -> Result<String, String> {
    if value.len() > 16384 || value.chars().any(|c| c.is_control()) {
        return Err("Magnet demasiado largo o con caracteres de control".into());
    }
    let uri = Url::parse(value).map_err(|_| "URI inválida")?;
    if uri.scheme() != "magnet"
        || uri.host_str().is_some()
        || !uri.path().is_empty()
        || uri.fragment().is_some()
    {
        return Err("Solo se permiten enlaces magnet válidos".into());
    }
    let mut valid = false;
    for (_, xt) in uri.query_pairs().filter(|(k, _)| k == "xt") {
        let ok = if let Some(h) = xt.strip_prefix("urn:btih:") {
            (h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit()))
                || (h.len() == 32
                    && data_encoding::BASE32
                        .decode(h.to_uppercase().as_bytes())
                        .is_ok())
        } else if let Some(h) = xt.strip_prefix("urn:btmh:1220") {
            h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())
        } else {
            false
        };
        if !ok {
            return Err("Infohash inválido".into());
        }
        valid = true;
    }
    if !valid {
        return Err("Falta un infohash BitTorrent válido".into());
    }
    Ok(uri.to_string())
}

pub fn merge(all: &mut Vec<Torrent>, incoming: Vec<Torrent>) {
    for item in incoming {
        if let Some(existing) = all.iter_mut().find(|r| r.id == item.id) {
            for source in item.sources {
                if !existing.sources.contains(&source) {
                    existing.sources.push(source);
                }
            }
        } else {
            all.push(item);
        }
    }
}

pub fn matches(title: &str, query: &str) -> bool {
    let title = title.to_lowercase().replace(['-', '_', '.'], " ");
    query
        .split_whitespace()
        .all(|word| title.contains(&word.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_uri() {
        for value in [
            "https://example.com",
            "file:///c:/foo",
            "magnet:?xt=urn:btih:bad",
            "magnet://evil?xt=urn:btih:0000000000000000000000000000000000000000",
            "magnet:?dn=hello",
            "magnet:?xt=urn:btih:0000000000000000000000000000000000000000\n",
        ] {
            assert!(validate_magnet(value).is_err(), "{value}");
        }
        assert!(validate_magnet(
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=Ubuntu"
        )
        .is_ok());
        assert!(validate_magnet(&format!("magnet:?xt=urn:btmh:1220{}", "a".repeat(64))).is_ok());
    }
    #[test]
    fn token_matching() {
        assert!(matches("Ubuntu-24.04-desktop-amd64.iso", "ubuntu desktop"));
        assert!(!matches("Debian", "ubuntu"));
    }
    #[test]
    fn dedupe_only_exact_hash() {
        let fixture = |hash: &str, source: &str| Torrent {
            id: hash.into(),
            title: "same title".into(),
            infohash_v1: hash.into(),
            magnet_uri: String::new(),
            size_bytes: None,
            published_at: None,
            seeds_reported: None,
            availability_status: "unknown".into(),
            availability_checked_at: None,
            sources: vec![source.into()],
            files: vec![],
            content_types: vec![],
            network: "bittorrent".into(),
            ed2k_uri: None,
            complete_sources_reported: None,
            discovered_at: String::new(),
        };
        let mut rows = vec![fixture("a", "ubuntu")];
        merge(
            &mut rows,
            vec![fixture("a", "debian"), fixture("b", "debian")],
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].sources, vec!["ubuntu", "debian"]);
    }
}
