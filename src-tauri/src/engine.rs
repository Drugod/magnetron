//! An unmodified, GPL-licensed Jackett process bundled with the app. No service install.
use crate::{
    model::Torrent,
    sources::{Batch, TorrentSource},
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::OnceCell;

#[derive(Clone, Serialize, Deserialize)]
pub struct Indexer {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub site_link: String,
    pub language: String,
}
pub struct Engine {
    executable: PathBuf,
    data: PathBuf,
    started: OnceCell<Connection>,
    child: Mutex<Option<Child>>,
    job: Mutex<Option<Job>>,
    configured: tokio::sync::RwLock<std::collections::HashSet<String>>,
    health: Mutex<std::collections::BTreeMap<String, Health>>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Health {
    failures: u32,
    retry_after: u64,
    successes: u32,
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub struct Connection {
    pub client: reqwest::Client,
    pub base: String,
    key: String,
    pub catalog: Vec<Indexer>,
}
#[cfg(windows)]
pub(crate) struct Job(isize);
#[cfg(windows)]
impl Job {
    pub(crate) fn attach(child: &Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err("No se pudo crear el grupo del motor".into());
            }
            let job = Job(handle as isize);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0
            {
                return Err("No se pudo asociar el motor al ciclo de vida de MAGNETRON".into());
            }
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0 as _);
        }
    }
}
#[cfg(not(windows))]
pub(crate) struct Job;
#[cfg(not(windows))]
impl Job {
    pub(crate) fn attach(_child: &Child) -> Result<Self, String> {
        Ok(Self)
    }
}
impl Engine {
    pub fn new(executable: PathBuf, data: PathBuf) -> Arc<Self> {
        let health = std::fs::read(data.join("source-health.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Arc::new(Self {
            executable,
            data,
            started: OnceCell::new(),
            child: Mutex::new(None),
            job: Mutex::new(None),
            configured: tokio::sync::RwLock::new(std::collections::HashSet::new()),
            health: Mutex::new(health),
        })
    }
    pub async fn connection(&self) -> Result<&Connection, String> {
        self.started.get_or_try_init(|| self.start()).await
    }
    async fn start(&self) -> Result<Connection, String> {
        if let Some(exe) = crate::engine_updates::selected(&self.data) {
            match self.start_at(&exe).await {
                Ok(c) => return Ok(c),
                Err(_) => {
                    if let Some(mut child) = self.child.lock().unwrap().take() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    self.job.lock().unwrap().take();
                    crate::engine_updates::reject(&self.data);
                }
            }
        }
        self.start_at(&self.executable).await
    }
    async fn start_at(&self, executable: &std::path::Path) -> Result<Connection, String> {
        if !executable.is_file() {
            return Err(
                "Falta el motor incluido. Usa el instalador o la carpeta portátil completa.".into(),
            );
        }
        // Reserve a free loopback port; the engine binds only 127.0.0.1.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        std::fs::create_dir_all(&self.data).map_err(|e| e.to_string())?;
        let mut random = [0; 32];
        rand::thread_rng().fill_bytes(&mut random);
        let key = hex::encode(random);
        rand::thread_rng().fill_bytes(&mut random);
        let password = hex::encode(random);
        let salted: Vec<u8> = format!("{password}{key}")
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let config = serde_json::json!({"Port":port,"LocalBindAddress":"127.0.0.1","AllowExternal":false,"AllowCORS":false,"APIKey":key,"AdminPassword":hex::encode(Sha512::digest(&salted)),"UpdateDisabled":true,"CacheEnabled":true,"CacheTtl":2100,"CacheMaxResultsPerIndexer":500,"FlareSolverrUrl":"","OmdbApiKey":""});
        std::fs::write(
            self.data.join("ServerConfig.json"),
            serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        drop(listener);
        let mut command = Command::new(executable);
        command
            .args([
                "--ListenPrivate",
                "--NoUpdates",
                "--NoRestart",
                "--DataFolder",
            ])
            .arg(&self.data)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("No se pudo arrancar el motor: {e}"))?;
        {
            match Job::attach(&child) {
                Ok(job) => *self.job.lock().map_err(|e| e.to_string())? = Some(job),
                Err(e) => {
                    let _ = child.kill();
                    return Err(e);
                }
            }
        }
        *self.child.lock().map_err(|e| e.to_string())? = Some(child);
        let base = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(45))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?;
        let mut ready = false;
        for _ in 0..100 {
            if client
                .post(format!("{base}/UI/Dashboard"))
                .form(&[("password", &password)])
                .send()
                .await
                .is_ok_and(|r| r.status().is_redirection())
            {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        if !ready {
            return Err("El motor no respondió al arrancar".into());
        }
        let catalog: Vec<Indexer> = client
            .get(format!("{base}/api/v2.0/indexers"))
            .send()
            .await
            .map_err(|_| "El motor no está disponible")?
            .error_for_status()
            .map_err(|_| "No se pudo acceder al catálogo")?
            .json()
            .await
            .map_err(|_| "Catálogo de conectores inválido")?;
        Ok(Connection {
            client,
            base,
            key,
            catalog,
        })
    }
    pub async fn catalog(&self) -> Result<Vec<Indexer>, String> {
        Ok(self
            .connection()
            .await?
            .catalog
            .iter()
            .filter(|i| i.kind == "public")
            .cloned()
            .collect())
    }
    pub async fn search(&self, id: &str, query: &str, batch: Batch) -> Result<usize, String> {
        let result =
            tokio::time::timeout(Duration::from_secs(40), self.search_inner(id, query, batch))
                .await
                .unwrap_or_else(|_| Err("La fuente tardó demasiado".into()));
        let mut health = self.health.lock().unwrap();
        let h = health.entry(id.to_owned()).or_default();
        if result.is_ok() {
            h.successes = h.successes.saturating_add(1);
            h.failures = 0;
            h.retry_after = 0
        } else {
            h.failures = h.failures.saturating_add(1);
            h.retry_after = now() + 60 * (1u64 << h.failures.min(10));
        }
        if let Ok(bytes) = serde_json::to_vec(&*health) {
            let _ = std::fs::write(self.data.join("source-health.json"), bytes);
        }
        result
    }
    pub async fn automatic_sources(&self) -> Result<Vec<String>, String> {
        let catalog = self.catalog().await?;
        let health = self.health.lock().unwrap();
        let mut ids: Vec<_> = catalog
            .into_iter()
            .filter(|i| health.get(&i.id).is_none_or(|h| h.retry_after <= now()))
            .map(|i| i.id)
            .collect();
        ids.sort_by_key(|id| {
            let successes = health.get(id).map_or(0, |h| h.successes);
            let preferred =
                ["torrentscsv", "knaben", "nyaasi", "animetosho-xyz"].contains(&id.as_str());
            (std::cmp::Reverse(successes), !preferred, id.clone())
        });
        Ok(ids)
    }
    async fn search_inner(&self, id: &str, query: &str, batch: Batch) -> Result<usize, String> {
        let c = self.connection().await?;
        if !c.catalog.iter().any(|i| i.id == id && i.kind == "public") {
            return Err("Fuente pública no compatible".into());
        }
        let configured = { self.configured.read().await.contains(id) };
        if !configured {
            let config_url = format!("{}/api/v2.0/indexers/{id}/config", c.base);
            let config: Value = c
                .client
                .get(&config_url)
                .send()
                .await
                .map_err(|_| "No se pudo consultar la fuente")?
                .json()
                .await
                .map_err(|_| "Configuración de fuente inválida")?;
            let response = c
                .client
                .post(&config_url)
                .json(&config)
                .send()
                .await
                .map_err(|_| "No se pudo activar la fuente")?;
            if !response.status().is_success() {
                return Err("La fuente requiere configuración, no responde o bloquea el acceso. No se eluden sus controles.".into());
            }
            self.configured.write().await.insert(id.to_string());
        }
        let response = c
            .client
            .get(format!(
                "{}/api/v2.0/indexers/{id}/results/torznab/api",
                c.base
            ))
            .query(&[
                ("apikey", c.key.as_str()),
                ("t", "search"),
                ("q", query),
                ("limit", "100"),
            ])
            .send()
            .await
            .map_err(|_| "La fuente no respondió dentro del plazo")?;
        if !response.status().is_success() {
            return Err(format!(
                "Fuente no disponible (HTTP {})",
                response.status().as_u16()
            ));
        }
        let mut response = response;
        let mut bytes = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Respuesta interrumpida")?
        {
            if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
                return Err("Respuesta de fuente demasiado grande".into());
            }
            bytes.extend(chunk)
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| "Respuesta no válida")?;
        let items = parse_results(text, &format!("indexer:{id}"))?;
        let n = items.len();
        batch(items);
        Ok(n)
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        if let Ok(child) = self.child.get_mut() {
            if let Some(child) = child {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        let _ = std::fs::remove_file(self.data.join("ServerConfig.json"));
    }
}

pub struct EngineSource {
    pub engine: Arc<Engine>,
    pub id: String,
}
impl TorrentSource for EngineSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn search<'a>(
        &'a self,
        _: &'a reqwest::Client,
        query: &'a str,
        batch: Batch,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            self.engine
                .search(self.id.trim_start_matches("indexer:"), query, batch)
                .await
        })
    }
}

pub fn parse_results(xml: &str, source: &str) -> Result<Vec<Torrent>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|_| "La fuente no devolvió XML válido")?;
    if doc.descendants().any(|n| n.has_tag_name("error")) {
        return Err("La fuente devolvió un error de consulta".into());
    }
    let mut result = vec![];
    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let text = |name: &str| {
            item.children()
                .find(|n| n.has_tag_name(name))
                .and_then(|n| n.text())
        };
        let attr = |name: &str| {
            item.children()
                .find(|n| n.tag_name().name() == "attr" && n.attribute("name") == Some(name))
                .and_then(|n| n.attribute("value"))
        };
        let Some(title) = text("title") else { continue };
        let hash = attr("infohash")
            .map(str::to_lowercase)
            .filter(|h| h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit()));
        let raw = attr("magneturl")
            .or_else(|| text("link").filter(|l| l.starts_with("magnet:")))
            .or_else(|| {
                item.children()
                    .find(|n| n.has_tag_name("enclosure"))
                    .and_then(|n| n.attribute("url"))
                    .filter(|l| l.starts_with("magnet:"))
            });
        let uri = if let Some(raw) = raw {
            match crate::model::validate_magnet(raw) {
                Ok(uri) => uri,
                Err(_) => continue,
            }
        } else if let Some(hash) = &hash {
            let mut uri = url::Url::parse("magnet:").unwrap();
            uri.query_pairs_mut()
                .append_pair("xt", &format!("urn:btih:{hash}"))
                .append_pair("dn", title);
            uri.to_string()
        } else {
            continue;
        };
        let hash = hash.or_else(|| {
            url::Url::parse(&uri)
                .ok()?
                .query_pairs()
                .find_map(|(k, v)| {
                    if k != "xt" {
                        return None;
                    }
                    let h = v.strip_prefix("urn:btih:")?;
                    if h.len() == 40 {
                        Some(h.to_lowercase())
                    } else {
                        data_encoding::BASE32
                            .decode(h.to_uppercase().as_bytes())
                            .ok()
                            .map(hex::encode)
                    }
                })
        });
        let v2_id = crate::v2::identity(&uri).ok().map(|(id, _)| id);
        if hash.is_none() && v2_id.is_none() {
            continue;
        }
        let hash = hash.unwrap_or_default();
        // Reject contradictory hashes instead of merging unrelated magnets.
        let parsed = url::Url::parse(&uri).unwrap();
        if let Some(h) = parsed.query_pairs().find_map(|(k, v)| {
            if k == "xt" {
                v.strip_prefix("urn:btih:").map(str::to_lowercase)
            } else {
                None
            }
        }) {
            if h.len() == 40 && h != hash {
                continue;
            }
        }
        let seeds = attr("seeders").and_then(|n| n.parse().ok());
        let mut content_types = vec![];
        for category in item
            .children()
            .filter(|n| n.tag_name().name() == "attr" && n.attribute("name") == Some("category"))
            .filter_map(|n| n.attribute("value")?.parse::<u32>().ok())
        {
            let kind = match category / 1000 {
                1 | 4 => Some("software"),
                2 | 5 => Some("video"),
                3 => Some("audio"),
                7 => Some("documents"),
                _ => None,
            };
            if let Some(kind) = kind {
                if !content_types.iter().any(|s| s == kind) {
                    content_types.push(kind.to_string());
                }
            }
        }
        let size = attr("size")
            .or_else(|| text("size"))
            .and_then(|n| n.parse().ok())
            .or_else(|| {
                item.children()
                    .find(|n| n.has_tag_name("enclosure"))
                    .and_then(|n| n.attribute("length"))
                    .and_then(|n| n.parse().ok())
            });
        result.push(Torrent {
            id: v2_id.unwrap_or_else(|| hash.clone()),
            infohash_v1: hash,
            title: title.to_string(),
            magnet_uri: uri,
            size_bytes: size,
            published_at: text("pubDate")
                .and_then(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
                .map(|d| d.to_rfc3339()),
            seeds_reported: seeds,
            availability_status: if seeds.is_some() {
                "reported"
            } else {
                "unknown"
            }
            .into(),
            availability_checked_at: None,
            sources: vec![source.into()],
            files: vec![],
            content_types,
            network: "bittorrent".into(),
            ed2k_uri: None,
            complete_sources_reported: None,
            discovered_at: chrono::Utc::now().to_rfc3339(),
        });
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn torznab_untrusted_and_reported() {
        let xml = r#"<rss xmlns:t="http://torznab.com/schemas/2015/feed"><channel><item><title>&lt;script&gt;bad&lt;/script&gt;</title><t:attr name="infohash" value="0123456789abcdef0123456789abcdef01234567"/><t:attr name="seeders" value="0"/></item><item><title>bad</title><link>file:///evil.exe</link></item></channel></rss>"#;
        let rows = parse_results(xml, "test").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "<script>bad</script>");
        assert_eq!(rows[0].availability_status, "reported");
        assert_eq!(rows[0].seeds_reported, Some(0));
    }
    #[test]
    fn preserves_content_categories() {
        let xml = r#"<rss xmlns:t="http://torznab.com/schemas/2015/feed"><channel><item><title>A film without extension</title><t:attr name="infohash" value="0123456789abcdef0123456789abcdef01234567"/><t:attr name="category" value="2040"/><t:attr name="category" value="2000"/></item><item><title>A book without extension</title><t:attr name="infohash" value="1123456789abcdef0123456789abcdef01234567"/><t:attr name="category" value="7020"/></item></channel></rss>"#;
        let rows = parse_results(xml, "test").unwrap();
        assert_eq!(rows[0].content_types, vec!["video"]);
        assert_eq!(rows[1].content_types, vec!["documents"]);
    }
    #[tokio::test]
    #[ignore = "Arranca el motor incluido y consulta fuentes públicas"]
    async fn live_engine() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let engine = Engine::new(
            crate::platform::dev_jackett(),
            root.join("artifacts/engine-smoke"),
        );
        let catalog = engine.catalog().await.unwrap();
        println!("Conectores públicos: {}", catalog.len());
        assert!(catalog.len() > 50);
        let mut found = 0;
        for id in ["torrentscsv", "knaben"] {
            let rows = Arc::new(Mutex::new(vec![]));
            let output = rows.clone();
            let r = engine
                .search(
                    id,
                    "big buck bunny",
                    Arc::new(move |batch| output.lock().unwrap().extend(batch)),
                )
                .await;
            println!("{id}: {r:?}");
            found += rows.lock().unwrap().len();
        }
        assert!(
            found > 0,
            "Ninguna fuente general devolvió los torrents públicos de prueba"
        );
    }
}
