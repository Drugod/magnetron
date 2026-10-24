use crate::{
    model::{self, Torrent},
    torrent,
};
use futures_util::StreamExt;
use reqwest::Client;
use scraper::{Html, Selector};
use std::{future::Future, pin::Pin, sync::Arc};
use url::Url;

pub type Batch = Arc<dyn Fn(Vec<Torrent>) + Send + Sync>;
pub trait TorrentSource: Send + Sync {
    fn id(&self) -> &str;
    fn search<'a>(
        &'a self,
        client: &'a Client,
        query: &'a str,
        batch: Batch,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>>;
}
pub struct OfficialCatalog {
    pub id: &'static str,
    pub roots: Vec<String>,
    pub discover_releases: bool,
}
pub fn registry() -> Vec<Box<dyn TorrentSource>> {
    vec![
        Box::new(OfficialCatalog {
            id: "ubuntu",
            roots: vec!["https://releases.ubuntu.com/".into()],
            discover_releases: true,
        }),
        Box::new(OfficialCatalog {
            id: "debian",
            roots: vec![
                "https://cdimage.debian.org/debian-cd/current/amd64/bt-cd/".into(),
                "https://cdimage.debian.org/debian-cd/current/amd64/bt-dvd/".into(),
                "https://cdimage.debian.org/debian-cd/current-live/amd64/bt-hybrid/".into(),
            ],
            discover_releases: false,
        }),
        Box::new(InternetArchive),
    ]
}
pub fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent("MAGNETRON/0.1 (official distribution catalog search)")
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 4 || !allowed(attempt.url()) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .connect_timeout(std::time::Duration::from_secs(8))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}
fn allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str().is_some_and(|host| {
            matches!(
                host,
                "releases.ubuntu.com" | "cdimage.debian.org" | "archive.org"
            ) || host.ends_with(".archive.org")
        })
        && url.username().is_empty()
        && url.password().is_none()
}
async fn fetch(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    let uri = Url::parse(url).map_err(|e| e.to_string())?;
    if !allowed(&uri) {
        return Err("Dominio de fuente no permitido".into());
    }
    let response = client
        .get(uri)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response
        .content_length()
        .is_some_and(|len| len > 8 * 1024 * 1024)
    {
        return Err("Respuesta demasiado grande".into());
    }
    let mut body = Vec::new();
    let mut chunks = response.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if body.len() + chunk.len() > 8 * 1024 * 1024 {
            return Err("Respuesta demasiado grande".into());
        }
        body.extend(chunk);
    }
    Ok(body)
}
pub struct InternetArchive;
impl TorrentSource for InternetArchive {
    fn id(&self) -> &'static str {
        "archive"
    }
    fn search<'a>(
        &'a self,
        client: &'a Client,
        query: &'a str,
        batch: Batch,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            let words = query
                .split_whitespace()
                .map(|word| format!("\"{}\"", word.replace(['"', '\\'], " ")))
                .collect::<Vec<_>>()
                .join(" AND ");
            let mut url = Url::parse("https://archive.org/advancedsearch.php").unwrap();
            url.query_pairs_mut()
                .append_pair(
                    "q",
                    &format!(
                        "({words}) AND -mediatype:collection AND -access-restricted-item:true"
                    ),
                )
                .append_pair("fl[]", "identifier")
                .append_pair("fl[]", "title")
                .append_pair("fl[]", "mediatype")
                .append_pair("rows", "20")
                .append_pair("output", "json");
            let json: serde_json::Value =
                serde_json::from_slice(&fetch(client, url.as_str()).await?)
                    .map_err(|_| "Respuesta de búsqueda de Internet Archive inválida")?;
            let docs = json
                .pointer("/response/docs")
                .and_then(|d| d.as_array())
                .ok_or("Internet Archive no devolvió una lista de resultados")?;
            let mut count = 0;
            let mut failures = 0;
            for doc in docs {
                let Some(identifier) = doc.get("identifier").and_then(|s| s.as_str()) else {
                    continue;
                };
                if !identifier
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                {
                    continue;
                }
                let result = async {
                    let metadata: serde_json::Value = serde_json::from_slice(
                        &fetch(
                            client,
                            &format!("https://archive.org/metadata/{identifier}"),
                        )
                        .await?,
                    )
                    .map_err(|_| "Metadatos inválidos")?;
                    let files = metadata
                        .get("files")
                        .and_then(|v| v.as_array())
                        .ok_or("Sin archivos públicos")?;
                    let Some(file) = files
                        .iter()
                        .filter_map(|f| f.get("name").and_then(|n| n.as_str()))
                        .find(|n| n.ends_with(".torrent"))
                    else {
                        return Ok(None);
                    };
                    let mut url = Url::parse("https://archive.org/download/").unwrap();
                    url.path_segments_mut()
                        .map_err(|_| "URL inválida")?
                        .pop_if_empty()
                        .push(identifier)
                        .push(file);
                    let mut item = torrent::parse(&fetch(client, url.as_str()).await?, "archive")?;
                    if let Some(title) = doc.get("title").and_then(|v| v.as_str()) {
                        item.title = title.to_string();
                    }
                    if let Some(kind) =
                        doc.get("mediatype")
                            .and_then(|v| v.as_str())
                            .and_then(|m| match m {
                                "movies" => Some("video"),
                                "audio" | "etree" => Some("audio"),
                                "texts" => Some("documents"),
                                "image" => Some("images"),
                                "software" => Some("software"),
                                _ => None,
                            })
                    {
                        item.content_types.push(kind.into());
                    }
                    Ok::<_, String>(Some(item))
                }
                .await;
                match result {
                    Ok(Some(item)) => {
                        count += 1;
                        batch(vec![item]);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        failures += 1;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
            if failures > 0 {
                Err(format!(
                    "{count} resultados; {failures} elementos sin metadatos accesibles"
                ))
            } else {
                Ok(count)
            }
        })
    }
}
fn links(data: &[u8], base: &str) -> Vec<Url> {
    let Ok(base) = Url::parse(base) else {
        return vec![];
    };
    let html = Html::parse_document(&String::from_utf8_lossy(data));
    let selector = Selector::parse("a[href]").unwrap();
    let mut result = vec![];
    for node in html.select(&selector) {
        if let Some(url) = node.value().attr("href").and_then(|h| base.join(h).ok()) {
            if allowed(&url) && url.host_str() == base.host_str() && !result.contains(&url) {
                result.push(url);
            }
        }
    }
    result
}
impl TorrentSource for OfficialCatalog {
    fn id(&self) -> &'static str {
        self.id
    }
    fn search<'a>(
        &'a self,
        client: &'a Client,
        query: &'a str,
        batch: Batch,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            let mut roots = self.roots.clone();
            if self.discover_releases {
                let raw = fetch(client, &roots[0]).await?;
                let mut releases: Vec<_> = links(&raw, &roots[0])
                    .into_iter()
                    .filter(|u| {
                        let name = u.path().trim_matches('/');
                        name.contains('.')
                            && name.chars().all(|c| c.is_ascii_digit() || c == '.')
                            && name.len() < 12
                    })
                    .collect();
                releases.sort_by_key(|u| {
                    u.path()
                        .trim_matches('/')
                        .split('.')
                        .map(|s| s.parse::<u32>().unwrap_or(0))
                        .collect::<Vec<_>>()
                });
                releases.reverse();
                // Bound discovery work, not the extensible connector registry.
                roots = releases.into_iter().take(6).map(Into::into).collect();
            }
            let mut count = 0;
            let mut failures = vec![];
            let mut seen = std::collections::HashSet::new();
            for root in roots {
                let raw = match fetch(client, &root).await {
                    Ok(raw) => raw,
                    Err(e) => {
                        failures.push(e);
                        continue;
                    }
                };
                for url in links(&raw, &root)
                    .into_iter()
                    .filter(|u| u.path().ends_with(".torrent"))
                {
                    let filename = url
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .unwrap_or("");
                    if !model::matches(filename, query) || !seen.insert(url.to_string()) {
                        continue;
                    }
                    match fetch(client, url.as_str())
                        .await
                        .and_then(|raw| torrent::parse(&raw, self.id))
                    {
                        Ok(item) => {
                            count += 1;
                            batch(vec![item]);
                        }
                        Err(e) => failures.push(e),
                    }
                }
            }
            if !failures.is_empty() {
                Err(format!(
                    "{count} resultados; {} peticiones fallidas: {}",
                    failures.len(),
                    failures[0]
                ))
            } else {
                Ok(count)
            }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_links_stay_on_host() {
        let urls=links(br#"<a href="test.torrent">x</a><a href="https://evil.test/x.torrent">x</a><a href="http://releases.ubuntu.com/x">x</a>"#,"https://releases.ubuntu.com/24.04/");
        assert_eq!(urls.len(), 1);
        assert!(urls[0].path().ends_with("test.torrent"));
    }
    #[tokio::test]
    #[ignore = "Requiere conexión a catálogos oficiales"]
    async fn live_catalogs() {
        let c = client().unwrap();
        for source in registry() {
            let items = Arc::new(std::sync::Mutex::new(vec![]));
            let output = items.clone();
            let query = if source.id() == "archive" {
                "big buck bunny"
            } else {
                "iso"
            };
            let result = source
                .search(
                    &c,
                    query,
                    Arc::new(move |batch| output.lock().unwrap().extend(batch)),
                )
                .await;
            println!("{}: {:?}", source.id(), result);
            assert!(
                !items.lock().unwrap().is_empty(),
                "{} no devolvió resultados",
                source.id()
            );
        }
    }
}
