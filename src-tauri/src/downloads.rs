use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session, SessionOptions};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::{Mutex as AsyncMutex, OnceCell},
    task::JoinHandle,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub state: String,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub download_speed: u64,
    pub upload_speed: u64,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub(crate) uri: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    use sha1::{Digest, Sha1};
    fn sandbox() -> PathBuf {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/download-tests")
            .join(format!("{:016x}", rand::random::<u64>()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn validates_links_and_windows_paths() {
        assert!(hash_from_magnet("file:///C:/foo").is_err());
        let (hash, uri) = hash_from_magnet(&format!(
            "magnet:?xt=urn:btih:{}&xs=file:///C:/foo",
            "a".repeat(40)
        ))
        .unwrap();
        assert_eq!(hash, "a".repeat(40));
        assert!(!uri.contains("file:"));
        for path in [
            "../evil",
            "C:/evil",
            "/evil",
            "a/CON.txt",
            "a/file:stream",
            "a/file.",
            "a/file?bad",
        ] {
            assert!(validate_file_path(Path::new(path)).is_err(), "{path}");
        }
        assert!(validate_file_path(Path::new("folder/manual.pdf")).is_ok());
    }
    #[tokio::test]
    async fn metadata_download_pause_restart_resume_and_cancel_keep_files() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let root = sandbox();
        let seed_dir = root.join("seed");
        std::fs::create_dir(&seed_dir).unwrap();
        let payload: Vec<u8> = (0..262144).map(|i| (i % 251) as u8).collect();
        std::fs::write(seed_dir.join("proof.bin"), &payload).unwrap();
        let hashes: Vec<u8> = payload
            .chunks(16384)
            .flat_map(|p| Sha1::digest(p).to_vec())
            .collect();
        let mut info = format!(
            "d6:lengthi{}e4:name9:proof.bin12:piece lengthi16384e6:pieces{}:",
            payload.len(),
            hashes.len()
        )
        .into_bytes();
        info.extend(hashes);
        info.push(b'e');
        let hash = hex::encode(Sha1::digest(&info));
        let mut torrent = b"d4:info".to_vec();
        torrent.extend(info);
        torrent.push(b'e');
        let seed = Session::new_with_opts(
            seed_dir.clone(),
            SessionOptions {
                dht: None,
                disable_trackers: true,
                disable_local_service_discovery: true,
                listen: Some(librqbit::ListenerOptions {
                    listen_addr: "127.0.0.1:0".parse().unwrap(),
                    ipv4_only: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let seed_torrent = seed
            .add_torrent(
                AddTorrent::from_bytes(torrent),
                Some(AddTorrentOptions {
                    overwrite: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap()
            .into_handle()
            .unwrap();
        seed_torrent.wait_until_initialized().await.unwrap();
        assert!(seed_torrent.stats().finished);
        let addr = seed.listen_addr().unwrap();
        let port = addr.port();
        let tracker = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tracker_addr = tracker.local_addr().unwrap();
        let tracker_task = tokio::spawn(async move {
            loop {
                let (mut s, _) = tracker.accept().await.unwrap();
                let mut req = vec![0; 8192];
                let _ = s.read(&mut req).await;
                let mut body = b"d8:intervali60e5:peers6:".to_vec();
                body.extend([127, 0, 0, 1]);
                body.extend(port.to_be_bytes());
                body.push(b'e');
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = s.write_all(header.as_bytes()).await;
                let _ = s.write_all(&body).await;
            }
        });
        let mut uri = url::Url::parse("magnet:").unwrap();
        uri.query_pairs_mut()
            .append_pair("xt", &format!("urn:btih:{hash}"))
            .append_pair("tr", &format!("http://{tracker_addr}/announce"));
        let manager = Downloads::new(root.join("state")).unwrap();
        let options = || SessionOptions {
            dht: None,
            disable_local_service_discovery: true,
            ratelimits: librqbit::limits::LimitsConfig {
                download_bps: std::num::NonZeroU32::new(65536),
                ..Default::default()
            },
            ..Default::default()
        };
        let session = Session::new_with_opts(root.join("unused"), options())
            .await
            .unwrap();
        assert!(manager.session.set(session).is_ok());
        let id = manager
            .add(
                uri.to_string(),
                "Prueba de transferencia".into(),
                root.clone(),
            )
            .await
            .unwrap();
        assert_eq!(id, hash);
        assert_eq!(manager.folder(&id).unwrap(), root.canonicalize().unwrap());
        assert!(!root.join("MAGNETRON").exists());
        tokio::time::timeout(Duration::from_secs(40), async {
            loop {
                let row = manager.list().pop().unwrap();
                assert_ne!(row.state, "error", "{:?}", row.error);
                if row.state == "downloading"
                    && row.completed_bytes > 0
                    && row.completed_bytes < row.total_bytes
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(30)).await;
            }
        })
        .await
        .expect("La transferencia local no empezó");
        manager.action(&id, "pause").await.unwrap();
        let paused = manager.list().pop().unwrap();
        assert_eq!(paused.state, "paused");
        assert!(paused.completed_bytes > 0 && paused.completed_bytes < paused.total_bytes);
        let file = PathBuf::from(&paused.folder).join("proof.bin");
        assert!(file.exists());
        manager.shutdown().await;
        drop(manager);
        // A damaged partial piece must be detected and downloaded again after restart.
        {
            use std::io::Write;
            let mut partial = std::fs::OpenOptions::new().write(true).open(&file).unwrap();
            partial.write_all(&[255; 32]).unwrap();
        }
        let manager = Downloads::new(root.join("state")).unwrap();
        assert_eq!(manager.list()[0].state, "paused");
        let session = Session::new_with_opts(root.join("unused"), options())
            .await
            .unwrap();
        assert!(manager.session.set(session).is_ok());
        manager.action(&id, "resume").await.unwrap();
        tokio::time::timeout(Duration::from_secs(40), async {
            loop {
                let row = manager.list().pop().unwrap();
                assert_ne!(row.state, "error", "{:?}", row.error);
                if row.state == "completed" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("La descarga local no se completó");
        assert_eq!(std::fs::read(&file).unwrap(), payload);
        assert_eq!(file, root.canonicalize().unwrap().join("proof.bin"));
        assert_eq!(manager.list()[0].completed_bytes, 262144);
        assert!(manager.handle(&id).is_none());
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let unlocked = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&file);
            assert!(
                unlocked.is_ok(),
                "A completed download must release the file"
            );
        }
        // Cancellation during metadata resolution is persistent and never deletes existing files.
        let cancelled = manager
            .add(
                format!("magnet:?xt=urn:btih:{}", "b".repeat(40)),
                "Cancelar".into(),
                root.clone(),
            )
            .await
            .unwrap();
        let keep = manager.folder(&cancelled).unwrap().join("keep.txt");
        std::fs::write(&keep, b"keep").unwrap();
        manager.action(&cancelled, "cancel").await.unwrap();
        assert!(keep.exists());
        assert_eq!(
            manager
                .list()
                .into_iter()
                .find(|r| r.id == cancelled)
                .unwrap()
                .state,
            "cancelled"
        );
        manager.remove(&cancelled, false).await.unwrap();
        assert!(keep.exists());
        manager.remove(&id, true).await.unwrap();
        assert!(!file.exists());
        assert!(keep.exists());
        assert!(root.is_dir());
        assert!(manager.list().is_empty());
        manager.shutdown().await;
        seed.stop().await;
        tracker_task.abort();
    }
    #[test]
    fn deletion_only_removes_payload_and_empty_subdirectories() {
        let root = sandbox().canonicalize().unwrap();
        std::fs::create_dir_all(root.join("album/disc")).unwrap();
        std::fs::write(root.join("album/disc/song.bin"), b"song").unwrap();
        std::fs::write(root.join("album/keep.txt"), b"keep").unwrap();
        assert!(delete_payload(&root, &[PathBuf::from("../outside")]).is_err());
        delete_payload(&root, &[PathBuf::from("album/disc/song.bin")]).unwrap();
        assert!(!root.join("album/disc").exists());
        assert!(root.join("album/keep.txt").is_file());
        assert!(root.is_dir());
    }
}
// Never recursively delete a destination: it may contain unrelated user files.
pub(crate) fn delete_payload(root: &Path, paths: &[PathBuf]) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    reject_links(root)?;
    let canonical = root.canonicalize().map_err(|e| e.to_string())?;
    if canonical != root {
        return Err(
            "La carpeta de descarga cambió de ubicación; no se borra ningún archivo".into(),
        );
    }
    let mut targets = vec![];
    let mut dirs = std::collections::BTreeSet::new();
    for path in paths {
        validate_file_path(path)?;
        let mut target = canonical.clone();
        for part in path.components() {
            target.push(part);
            reject_links(&target)?;
        }
        if target.exists() {
            if !target.is_file()
                || !target
                    .canonicalize()
                    .map_err(|e| e.to_string())?
                    .starts_with(&canonical)
            {
                return Err("Archivo fuera del destino de descarga".into());
            }
            targets.push(target.clone());
        }
        let mut parent = target.parent();
        while let Some(dir) = parent {
            if dir == canonical {
                break;
            }
            if !dir.starts_with(&canonical) {
                return Err("Carpeta fuera del destino".into());
            }
            dirs.insert(dir.to_owned());
            parent = dir.parent();
        }
    }
    for file in targets {
        std::fs::remove_file(file).map_err(|e| e.to_string())?;
    }
    let mut dirs: Vec<_> = dirs.into_iter().collect();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in dirs {
        if dir.is_dir()
            && std::fs::read_dir(&dir)
                .map_err(|e| e.to_string())?
                .next()
                .is_none()
        {
            std::fs::remove_dir(dir).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub struct Downloads {
    dir: PathBuf,
    records: Mutex<BTreeMap<String, Download>>,
    session: OnceCell<Arc<Session>>,
    control: AsyncMutex<()>,
    jobs: Mutex<BTreeMap<String, JoinHandle<()>>>,
    limits: Mutex<librqbit::limits::LimitsConfig>,
}
fn hash_from_magnet(uri: &str) -> Result<(String, String), String> {
    let safe = crate::model::validate_magnet(uri)?;
    let parsed = url::Url::parse(&safe).map_err(|e| e.to_string())?;
    let hash = parsed
        .query_pairs()
        .find_map(|(k, v)| {
            if k == "xt" {
                v.strip_prefix("urn:btih:").map(str::to_owned)
            } else {
                None
            }
        })
        .ok_or("Se necesita un magnet BitTorrent v1")?;
    let hash = if hash.len() == 32 {
        hex::encode(
            data_encoding::BASE32
                .decode(hash.to_uppercase().as_bytes())
                .map_err(|_| "Hash inválido")?,
        )
    } else {
        hash.to_lowercase()
    };
    // Only identity, display name and supported tracker URLs reach the engine.
    let mut clean = url::Url::parse("magnet:").unwrap();
    clean
        .query_pairs_mut()
        .append_pair("xt", &format!("urn:btih:{hash}"));
    for (k, v) in parsed.query_pairs() {
        if k == "dn" {
            clean.query_pairs_mut().append_pair("dn", &v);
        }
        if k == "tr"
            && url::Url::parse(&v).is_ok_and(|u| {
                matches!(u.scheme(), "http" | "https" | "udp")
                    && u.username().is_empty()
                    && u.password().is_none()
            })
        {
            clean.query_pairs_mut().append_pair("tr", &v);
        }
    }
    Ok((hash, clean.to_string()))
}
pub(crate) fn validate_file_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("Ruta de archivo vacía".into());
    }
    for component in path.components() {
        let Component::Normal(name) = component else {
            return Err("Ruta de torrent no segura".into());
        };
        let name = name.to_str().ok_or("Nombre no compatible con Windows")?;
        if name.is_empty()
            || name.ends_with(['.', ' '])
            || name
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*\\/".contains(c))
        {
            return Err("Nombre de archivo no compatible con Windows".into());
        }
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && (b'1'..=b'9').contains(&stem.as_bytes()[3]))
        {
            return Err("Nombre reservado de Windows".into());
        }
    }
    Ok(())
}
pub(crate) fn reject_links(path: &Path) -> Result<(), String> {
    if let Ok(m) = std::fs::symlink_metadata(path) {
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & 0x400 != 0 {
                return Err("La carpeta contiene un enlace o unión de directorios".into());
            }
        }
        if m.file_type().is_symlink() {
            return Err("La carpeta contiene un enlace".into());
        }
    }
    Ok(())
}
impl Downloads {
    pub(crate) fn current_session(&self) -> Option<Arc<Session>> {
        self.session.get().cloned()
    }
    pub async fn remove(self: &Arc<Self>, id: &str, delete_files: bool) -> Result<(), String> {
        let _guard = self.control.lock().await;
        let record = self
            .records
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        if let Some(job) = self.jobs.lock().unwrap().remove(id) {
            job.abort();
        }
        if let Some(handle) = self.handle(id) {
            self.session
                .get()
                .unwrap()
                .delete(librqbit::api::TorrentIdOrHash::Id(handle.id()), false)
                .await
                .map_err(|e| e.to_string())?;
        }
        let metadata_path = self.dir.join("metadata").join(format!("{id}.torrent"));
        if delete_files && metadata_path.is_file() {
            let bytes = std::fs::read(&metadata_path).map_err(|e| e.to_string())?;
            let meta = librqbit::torrent_from_bytes(&bytes).map_err(|e| e.to_string())?;
            if meta.info_hash.as_string() != id {
                return Err("Metadatos de otra descarga".into());
            }
            let info = meta.info.data.validate().map_err(|e| e.to_string())?;
            let paths = info
                .iter_file_details()
                .map(|f| f.filename.to_pathbuf())
                .collect::<Vec<_>>();
            delete_payload(std::path::Path::new(&record.folder), &paths)?;
        }
        self.records.lock().unwrap().remove(id);
        self.save()?;
        if metadata_path.exists() {
            std::fs::remove_file(metadata_path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn new(dir: PathBuf) -> Result<Arc<Self>, String> {
        std::fs::create_dir_all(dir.join("metadata")).map_err(|e| e.to_string())?;
        let file = dir.join("downloads.json");
        let mut records: BTreeMap<String, Download> = if file.exists() {
            serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
                .map_err(|e| format!("No se pudo leer la lista de descargas: {e}"))?
        } else {
            BTreeMap::new()
        };
        for record in records.values_mut() {
            if !matches!(record.state.as_str(), "completed" | "cancelled") {
                record.state = "paused".into();
            }
            record.download_speed = 0;
            record.upload_speed = 0;
        }
        Ok(Arc::new(Self {
            dir,
            records: Mutex::new(records),
            session: OnceCell::new(),
            control: AsyncMutex::new(()),
            jobs: Mutex::new(BTreeMap::new()),
            limits: Mutex::new(Default::default()),
        }))
    }
    fn save(&self) -> Result<(), String> {
        let records = self.records.lock().map_err(|e| e.to_string())?;
        crate::secure_fs::atomic_private(
            &self.dir.join("downloads.json"),
            &serde_json::to_vec(&*records).map_err(|e| e.to_string())?,
        )
    }
    pub(crate) async fn session(&self) -> Result<&Arc<Session>, String> {
        self.session
            .get_or_try_init(|| async {
                let ratelimits = *self.limits.lock().unwrap();
                Session::new_with_opts(
                    self.dir.join("unused"),
                    SessionOptions {
                        ratelimits,
                        dht: Some(librqbit::DhtSessionConfig {
                            persistence: None,
                            ..Default::default()
                        }),
                        disable_local_service_discovery: true,
                        peer_limit: Some(60),
                        concurrent_init_limit: Some(2),
                        client_name_and_version: Some("MAGNETRON/0.4.0".into()),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| format!("No se pudo iniciar BitTorrent: {e:#}"))
            })
            .await
    }
    pub fn set_limits(&self, down: u32, up: u32) {
        let config = librqbit::limits::LimitsConfig {
            download_bps: std::num::NonZeroU32::new(down.saturating_mul(1024)),
            upload_bps: std::num::NonZeroU32::new(up.saturating_mul(1024)),
        };
        let mut old = self.limits.lock().unwrap();
        if *old == config {
            return;
        }
        *old = config;
        if let Some(session) = self.session.get() {
            session.ratelimits.set_download_bps(config.download_bps);
            session.ratelimits.set_upload_bps(config.upload_bps);
        }
    }
    fn handle(&self, id: &str) -> Option<Arc<librqbit::ManagedTorrent>> {
        self.session.get()?.with_torrents(|t| {
            for (_, h) in t {
                if h.info_hash().as_string() == id {
                    return Some(h.clone());
                }
            }
            None
        })
    }
    pub fn list(&self) -> Vec<Download> {
        let mut records = self.records.lock().unwrap();
        for record in records.values_mut() {
            if let Some(handle) = self.handle(&record.id) {
                let s = handle.stats();
                record.completed_bytes = s.progress_bytes;
                record.total_bytes = s.total_bytes;
                record.download_speed = s.live.as_ref().map_or(0, |l| l.download_speed.as_bytes());
                record.upload_speed = s.live.as_ref().map_or(0, |l| l.upload_speed.as_bytes());
                if let Some(e) = s.error {
                    record.error = Some(e);
                    record.state = "error".into();
                } else if matches!(record.state.as_str(), "downloading" | "checking") {
                    record.state =
                        if matches!(s.state, librqbit::TorrentStatsState::Initializing { .. }) {
                            "checking"
                        } else {
                            "downloading"
                        }
                        .into();
                }
            }
        }
        records
            .values()
            .cloned()
            .map(|mut r| {
                r.uri.clear();
                r
            })
            .collect()
    }
    pub async fn add(
        self: &Arc<Self>,
        uri: String,
        name: String,
        root: PathBuf,
    ) -> Result<String, String> {
        let (id, uri) = hash_from_magnet(&uri)?;
        let _guard = self.control.lock().await;
        if self.records.lock().unwrap().contains_key(&id) {
            return Ok(id);
        }
        let root = root
            .canonicalize()
            .map_err(|_| "Elige una carpeta existente")?;
        if !root.is_dir() {
            return Err("El destino no es una carpeta".into());
        }
        reject_links(&root)?;
        let folder = root;
        self.records.lock().unwrap().insert(
            id.clone(),
            Download {
                id: id.clone(),
                uri,
                name: name.chars().take(500).collect(),
                folder: folder.to_str().ok_or("Ruta no compatible")?.into(),
                state: "resolving".into(),
                completed_bytes: 0,
                total_bytes: 0,
                download_speed: 0,
                upload_speed: 0,
                error: None,
            },
        );
        if let Err(e) = self.save() {
            self.records.lock().unwrap().remove(&id);
            return Err(e);
        }
        self.spawn(id.clone(), false);
        Ok(id)
    }
    fn spawn(self: &Arc<Self>, id: String, resume: bool) {
        let this = self.clone();
        let key = id.clone();
        let task = tokio::spawn(async move {
            if let Err(e) = this.run(&id, resume).await {
                let _guard = this.control.lock().await;
                if let Some(r) = this.records.lock().unwrap().get_mut(&id) {
                    if !matches!(r.state.as_str(), "paused" | "cancelled") {
                        r.state = "error".into();
                        r.error = Some(e);
                    }
                }
                let _ = this.save();
            }
        });
        if let Some(old) = self.jobs.lock().unwrap().insert(key, task) {
            old.abort();
        }
    }
    async fn run(&self, id: &str, resume: bool) -> Result<(), String> {
        let record = self
            .records
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        let session = self.session().await?;
        let handle = if let Some(h) = self.handle(id) {
            h
        } else {
            let metadata_path = self.dir.join("metadata").join(format!("{id}.torrent"));
            let input = if metadata_path.exists() {
                AddTorrent::from_bytes(std::fs::read(&metadata_path).map_err(|e| e.to_string())?)
            } else {
                AddTorrent::from_url(record.uri.clone())
            };
            let response = tokio::time::timeout(
                Duration::from_secs(120),
                session.add_torrent(
                    input,
                    Some(AddTorrentOptions {
                        list_only: true,
                        output_folder: Some(record.folder.clone()),
                        ..Default::default()
                    }),
                ),
            )
            .await
            .map_err(|_| "No llegaron los metadatos. Puedes reintentar la descarga.")?
            .map_err(|e| format!("No se obtuvieron metadatos: {e:#}"))?;
            let AddTorrentResponse::ListOnly(metadata) = response else {
                return Err("Respuesta inesperada del motor".into());
            };
            if metadata.info_hash.as_string() != id {
                return Err("El hash de los metadatos no coincide".into());
            }
            let mut names = std::collections::HashSet::new();
            let selected = PathBuf::from(&record.folder);
            reject_links(&selected)?;
            if selected
                .canonicalize()
                .map_err(|_| "La carpeta de descarga ya no está disponible")?
                != selected
            {
                return Err(
                    "La carpeta de descarga ha cambiado de ubicación o contiene una unión".into(),
                );
            }
            for file in metadata.info.iter_file_details() {
                let path = file.filename.to_pathbuf();
                validate_file_path(&path)?;
                if !names.insert(path.to_string_lossy().to_lowercase()) {
                    return Err("El torrent contiene nombres duplicados en Windows".into());
                }
                let mut destination = PathBuf::from(&record.folder);
                reject_links(&destination)?;
                for c in path.components() {
                    destination.push(c);
                    reject_links(&destination)?;
                }
                if !resume && destination.exists() {
                    return Err(format!(
                        "Ya existe {}. No se sobrescribe; cambia su nombre o elige otra carpeta",
                        destination.display()
                    ));
                }
            }
            std::fs::write(&metadata_path, &metadata.torrent_bytes).map_err(|e| e.to_string())?;
            session
                .add_torrent(
                    AddTorrent::from_bytes(metadata.torrent_bytes),
                    Some(AddTorrentOptions {
                        paused: true,
                        overwrite: resume,
                        output_folder: Some(record.folder.clone()),
                        initial_peers: Some(metadata.seen_peers),
                        ..Default::default()
                    }),
                )
                .await
                .map_err(|e| format!("No se pudo preparar la descarga: {e:#}"))?
                .into_handle()
                .ok_or("No se pudo crear la descarga")?
        };
        if let Some(r) = self.records.lock().unwrap().get_mut(id) {
            if r.state == "resolving" {
                r.state = "checking".into();
            }
        }
        handle
            .wait_until_initialized()
            .await
            .map_err(|e| format!("No se pudieron comprobar los archivos: {e:#}"))?;
        {
            let _guard = self.control.lock().await;
            if !self
                .records
                .lock()
                .unwrap()
                .get(id)
                .is_some_and(|r| matches!(r.state.as_str(), "resolving" | "checking"))
            {
                return Ok(());
            }
            session
                .unpause(&handle)
                .await
                .map_err(|e| format!("No se pudo iniciar: {e:#}"))?;
            let mut records = self.records.lock().unwrap();
            let r = records.get_mut(id).unwrap();
            r.name = handle.name().unwrap_or_else(|| r.name.clone());
            r.state = "downloading".into();
            drop(records);
            self.save()?;
        }
        handle
            .wait_until_completed()
            .await
            .map_err(|e| format!("Error de descarga: {e:#}"))?;
        let _guard = self.control.lock().await;
        let stats = handle.stats();
        // Removing the torrent without deleting its files also releases every file
        // handle. A paused torrent still retains its storage and locks on Windows.
        session
            .delete(librqbit::api::TorrentIdOrHash::Id(handle.id()), false)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(r) = self.records.lock().unwrap().get_mut(id) {
            r.state = "completed".into();
            r.completed_bytes = stats.progress_bytes;
            r.total_bytes = stats.total_bytes;
            r.download_speed = 0;
            r.upload_speed = 0;
            r.error = None;
        }
        self.save()
    }
    pub async fn action(self: &Arc<Self>, id: &str, action: &str) -> Result<(), String> {
        let _guard = self.control.lock().await;
        if !matches!(action, "pause" | "resume" | "cancel") {
            return Err("Acción desconocida".into());
        }
        let old = self
            .records
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        if old.state == "completed" {
            return Err("La descarga ya ha terminado".into());
        }
        if action == "resume" && !matches!(old.state.as_str(), "paused" | "error" | "cancelled") {
            return Err("La descarga ya está en curso".into());
        }
        if (action == "pause" && old.state == "paused")
            || (action == "cancel" && old.state == "cancelled")
        {
            return Ok(());
        }
        if let Some(job) = self.jobs.lock().unwrap().remove(id) {
            job.abort();
        }
        if let Some(handle) = self.handle(id) {
            if action == "cancel" || (action == "resume" && old.state == "error") {
                let stats = handle.stats();
                if let Some(r) = self.records.lock().unwrap().get_mut(id) {
                    r.completed_bytes = stats.progress_bytes;
                    r.total_bytes = stats.total_bytes;
                }
                self.session
                    .get()
                    .unwrap()
                    .delete(librqbit::api::TorrentIdOrHash::Id(handle.id()), false)
                    .await
                    .map_err(|e| e.to_string())?;
            } else if action == "pause" {
                let stats = handle.stats();
                if let Some(r) = self.records.lock().unwrap().get_mut(id) {
                    r.completed_bytes = stats.progress_bytes;
                    r.total_bytes = stats.total_bytes;
                }
                self.session
                    .get()
                    .unwrap()
                    .delete(librqbit::api::TorrentIdOrHash::Id(handle.id()), false)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        if let Some(r) = self.records.lock().unwrap().get_mut(id) {
            r.state = match action {
                "resume" => "resolving",
                "pause" => "paused",
                _ => "cancelled",
            }
            .into();
            r.download_speed = 0;
            r.upload_speed = 0;
            r.error = None;
        }
        self.save()?;
        if action == "resume" {
            self.spawn(id.into(), true)
        }
        Ok(())
    }
    pub fn folder(&self, id: &str) -> Result<PathBuf, String> {
        self.records
            .lock()
            .unwrap()
            .get(id)
            .map(|r| PathBuf::from(&r.folder))
            .ok_or("Descarga no encontrada".into())
    }
    pub async fn shutdown(&self) {
        let _guard = self.control.lock().await;
        let _ = self.list();
        for (_, job) in std::mem::take(&mut *self.jobs.lock().unwrap()) {
            job.abort();
        }
        if let Some(session) = self.session.get() {
            let _ = tokio::time::timeout(Duration::from_secs(8), session.stop()).await;
        }
        for r in self.records.lock().unwrap().values_mut() {
            if !matches!(r.state.as_str(), "completed" | "cancelled") {
                r.state = "paused".into();
            }
            r.download_speed = 0;
            r.upload_speed = 0;
        }
        let _ = self.save();
    }
}
