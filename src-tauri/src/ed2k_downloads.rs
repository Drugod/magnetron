//! Downloads through a private, unmodified aMuled child process.
use crate::{
    downloads::Download,
    ec::{Client, Tag},
    engine::Job,
};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Default, Serialize, Deserialize)]
struct Store {
    root: Option<PathBuf>,
    records: BTreeMap<String, Download>,
}
struct Running {
    child: Child,
    _job: Job,
    client: Client,
}
impl Running {
    async fn stop(mut self) {
        let _ = self.client.request(0x4b, vec![]).await;
        let _ = self.client.request(8, vec![]).await;
        for _ in 0..40 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub struct Downloads {
    executable: PathBuf,
    data: PathBuf,
    store: Mutex<Store>,
    running: AsyncMutex<Option<Running>>,
    control: AsyncMutex<()>,
    monitor: Mutex<Option<tokio::task::JoinHandle<()>>>,
    kad_users: std::sync::atomic::AtomicUsize,
    stop_search: std::sync::atomic::AtomicBool,
    search_control: AsyncMutex<()>,
    limits: Mutex<(u32, u32)>,
    shared_paths: Mutex<Vec<PathBuf>>,
}
struct KadLease(Arc<Downloads>);
impl Drop for KadLease {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering::SeqCst;
        self.0.kad_users.fetch_sub(1, SeqCst);
        self.0.stop_search.store(true, SeqCst);
    }
}
fn safe_dir(path: &Path) -> Result<PathBuf, String> {
    // Refuse reparse points in every existing component of the selected tree.
    let mut current = Some(path);
    while let Some(p) = current {
        if !p.as_os_str().is_empty() && p.exists() {
            let m = std::fs::symlink_metadata(p).map_err(|e| e.to_string())?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return Err("La carpeta no puede contener enlaces o uniones".into());
                }
            }
            if m.file_type().is_symlink() {
                return Err("La carpeta no puede ser un enlace".into());
            }
        }
        current = p.parent();
    }
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    std::fs::canonicalize(path).map_err(|e| e.to_string())
}
fn filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(160)
        .collect();
    let s = s.trim_matches([' ', '.']);
    let stem = s.split('.').next().unwrap_or("").to_uppercase();
    if s.is_empty()
        || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4)
    {
        format!("archivo_{s}")
    } else {
        s.into()
    }
}
fn ini_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let plain = if let Some(p) = raw.strip_prefix("\\\\?\\UNC\\") {
        format!("//{p}")
    } else {
        raw.strip_prefix("\\\\?\\").unwrap_or(&raw).to_string()
    };
    plain.replace('\\', "/")
}
#[cfg(test)]
fn print_startup_summary(data: &Path) {
    use std::io::Read;
    for name in ["logfile", "amule.log"] {
        let Ok(file) = std::fs::File::open(data.join(name)) else {
            continue;
        };
        let mut bytes = Vec::new();
        let _ = file.take(65536).read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).to_lowercase();
        // Only fixed category names and booleans reach CI; never print log text or values.
        for (category, marker) in [
            ("permission_error", "permission denied"),
            ("socket_error", "socket"),
            ("external_connections", "external connection"),
            ("server_list", "server.met"),
            ("ip_filter", "ipfilter"),
            ("download_attempt", "download"),
            ("fatal_error", "fatal"),
            ("initialising", "initialis"),
        ] {
            eprintln!(
                "aMuled startup category {category}: {}",
                text.contains(marker)
            );
        }
        eprintln!("aMuled startup log bytes inspected: {}", bytes.len());
        return;
    }
    eprintln!("aMuled startup log created: false");
}
impl Downloads {
    pub fn new(executable: PathBuf, data: PathBuf) -> Result<Arc<Self>, String> {
        std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
        let file = data.join("downloads.json");
        let mut store: Store = if file.exists() {
            serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        } else {
            Store::default()
        };
        for r in store.records.values_mut() {
            if !matches!(r.state.as_str(), "completed" | "cancelled") {
                r.state = "paused".into()
            }
            r.download_speed = 0;
            r.upload_speed = 0;
        }
        Ok(Arc::new(Self {
            executable,
            data,
            store: Mutex::new(store),
            running: AsyncMutex::new(None),
            control: AsyncMutex::new(()),
            monitor: Mutex::new(None),
            kad_users: std::sync::atomic::AtomicUsize::new(0),
            stop_search: std::sync::atomic::AtomicBool::new(false),
            search_control: AsyncMutex::new(()),
            limits: Mutex::new((0, 0)),
            shared_paths: Mutex::new(vec![]),
        }))
    }
    fn save(&self) -> Result<(), String> {
        let data = serde_json::to_vec(&*self.store.lock().unwrap()).map_err(|e| e.to_string())?;
        let tmp = self.data.join("downloads.json.tmp");
        std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
        std::fs::rename(tmp, self.data.join("downloads.json")).map_err(|e| e.to_string())
    }
    pub fn list(&self) -> Vec<Download> {
        self.store
            .lock()
            .unwrap()
            .records
            .values()
            .cloned()
            .map(|mut r| {
                r.uri.clear();
                r
            })
            .collect()
    }
    pub fn folder(&self, id: &str) -> Result<PathBuf, String> {
        self.store
            .lock()
            .unwrap()
            .records
            .get(id)
            .map(|r| PathBuf::from(&r.folder))
            .ok_or_else(|| "Descarga eD2k no encontrada".into())
    }
    pub async fn search_kad(
        self: &Arc<Self>,
        query: &str,
        batch: crate::sources::Batch,
    ) -> Result<usize, String> {
        let _search = self.search_control.lock().await;
        self.kad_users
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let _lease = KadLease(self.clone());
        {
            let _guard = self.control.lock().await;
            if self.store.lock().unwrap().root.is_none() {
                let root = safe_dir(&self.data.join("search-workspace"))?;
                self.store.lock().unwrap().root = Some(root);
                self.save()?;
            }
            // Download contacts before starting the daemon, so it loads them once.
            if !crate::kad::ready(&self.data) {
                {
                    let mut r = self.running.lock().await;
                    if let Some(engine) = r.as_mut() {
                        let _ = engine.client.request(0x49, vec![]).await;
                    }
                }
                crate::kad::bootstrap(&self.data).await?;
            }
            self.start().await?;
            let mut running = self.running.lock().await;
            let c = &mut running.as_mut().unwrap().client;
            self.stop_search
                .store(false, std::sync::atomic::Ordering::SeqCst);
            let _ = c.request(0x27, vec![]).await;
            c.request(0x48, vec![]).await?;
        }
        self.watch();
        let ready = tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                let (_, state) = self
                    .running
                    .lock()
                    .await
                    .as_mut()
                    .ok_or("Motor eD2k detenido")?
                    .client
                    .request(0xb, vec![])
                    .await?;
                if state.iter().any(|t| t.id == 5 && t.num() & 4 != 0) {
                    return Ok::<(), String>(());
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .map_err(|_| "Kad sigue conectando; prueba de nuevo en unos momentos".to_string());
        ready??;
        {
            let tag = Tag {
                id: 0x701,
                kind: 2,
                data: vec![2],
                children: vec![Tag::string(0x702, query)],
            };
            self.running
                .lock()
                .await
                .as_mut()
                .ok_or("Motor eD2k detenido")?
                .client
                .request(0x26, vec![tag])
                .await?;
        }
        let mut seen = std::collections::HashSet::new();
        for _ in 0..35 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let (_, tags) = self
                .running
                .lock()
                .await
                .as_mut()
                .ok_or("Motor eD2k detenido")?
                .client
                .request(0x28, vec![])
                .await?;
            let fresh: Vec<_> = crate::kad::results(&tags)
                .into_iter()
                .filter(|t| seen.insert(t.id.clone()))
                .collect();
            if !fresh.is_empty() {
                batch(fresh)
            }
        }
        Ok(seen.len())
    }
    async fn start(&self) -> Result<(), String> {
        let mut running = self.running.lock().await;
        if running.is_some() {
            return Ok(());
        }
        if !self.executable.is_file() {
            return Err("Falta el motor eD2k incluido con MAGNETRON".into());
        }
        let root = self
            .store
            .lock()
            .unwrap()
            .root
            .clone()
            .ok_or("No hay carpeta de descargas eD2k")?;
        if safe_dir(&root)? != root {
            return Err("La carpeta temporal eD2k ha cambiado".into());
        }
        let parts = safe_dir(&root.join("parts"))?;
        let incoming = safe_dir(&root.join("incoming"))?;
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| e.to_string())?
            .local_addr()
            .unwrap()
            .port();
        let peer_port = std::net::TcpListener::bind("0.0.0.0:0")
            .map_err(|e| e.to_string())?
            .local_addr()
            .unwrap()
            .port();
        let udp_port = std::net::UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| e.to_string())?
            .local_addr()
            .unwrap()
            .port();
        let pass = hex::encode(Md5::digest(rand::random::<[u8; 32]>()));
        let (down, up) = *self.limits.lock().unwrap();
        // Only engine-owned directories are shared. Completed files are moved out.
        let config=format!("[eMule]\nNick=MAGNETRON\nNewVersionCheck=0\nGeoIPEnabled=0\nGeoIPAutoUpdate=0\nServerlist=0\nPort={peer_port}\nUDPPort={udp_port}\nAutoconnect=0\nReconnect=1\nUPnPEnabled=0\nConnectToKad=1\nConnectToED2K=1\nTempDir={}\nIncomingDir={}\nAddNewFilesPaused=1\nStartNextFile=0\nOnlineSignature=0\nShareHiddenFiles=0\nMaxUpload=100\n[ExternalConnect]\nAcceptExternalConnections=1\nECAddress=127.0.0.1\nECPort={port}\nECPassword={pass}\nUPnPECEnabled=0\n[WebServer]\nEnabled=0\nUPnPWebServerEnabled=0\n",ini_path(&parts),ini_path(&incoming));
        #[cfg(test)]
        let config = config
            .replace("Nick=MAGNETRON", "Nick=MAGNETRON\nFilterLanIPs=0")
            .replace(
                "ConnectToKad=1",
                if self.kad_users.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                    "ConnectToKad=1"
                } else {
                    "ConnectToKad=0"
                },
            );
        let config = config.replace(
            "MaxUpload=100",
            &format!("MaxUpload={up}\nMaxDownload={down}"),
        );
        std::fs::write(self.data.join("amule.conf"), config).map_err(|e| e.to_string())?;
        // Never reload stale shares before the current explicit selection is applied.
        for file in [
            "shareddir.dat",
            "shareddir-explicit.dat",
            "shareddir-recursive.dat",
        ] {
            std::fs::write(self.data.join(file), b"").map_err(|e| e.to_string())?;
        }
        // An initial server.met avoids aMule's first-run fallback download and
        // connection before MAGNETRON has restored the user's paused queue.
        if !self.data.join("server.met").exists() {
            let mut met = vec![0xe0];
            met.extend_from_slice(&3u32.to_le_bytes());
            for (ip, port) in [
                ([176, 125, 242, 230], 5555u16),
                ([85, 17, 116, 222], 6082),
                ([77, 42, 68, 79], 4232),
            ] {
                met.extend_from_slice(&ip);
                met.extend_from_slice(&port.to_le_bytes());
                met.extend_from_slice(&0u32.to_le_bytes());
            }
            std::fs::write(self.data.join("server.met"), met).map_err(|e| e.to_string())?;
        }
        let mut command = Command::new(&self.executable);
        command
            .arg("--config-dir")
            .arg(&self.data)
            .current_dir(self.executable.parent().unwrap())
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
            .map_err(|e| format!("No se pudo iniciar eD2k: {e}"))?;
        #[cfg(windows)]
        let job = match Job::attach(&child) {
            Ok(j) => j,
            Err(e) => {
                let _ = child.kill();
                return Err(e);
            }
        };
        #[cfg(not(windows))]
        let job = Job;
        let mut connection = None;
        let mut last_error = String::new();
        for _ in 0..80 {
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                #[cfg(test)]
                print_startup_summary(&self.data);
                return Err("El motor eD2k no pudo arrancar. Cierra otras instancias de aMule y vuelve a intentarlo".into());
            }
            match Client::connect(port, &pass).await {
                Ok(c) => {
                    connection = Some(c);
                    break;
                }
                Err(e) => last_error = e,
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let Some(mut client) = connection else {
            let _ = child.kill();
            let _ = child.wait();
            #[cfg(test)]
            print_startup_summary(&self.data);
            return Err(format!("No se pudo conectar con eD2k: {last_error}"));
        };
        let (_, prefs) = client
            .request(0x3f, vec![Tag::number(0x1000, 0x200)])
            .await?;
        let directories = prefs
            .iter()
            .find(|t| t.id == 0x1a00)
            .ok_or("El motor eD2k no informó sus carpetas")?;
        for (tag, expected) in [(0x1a01, &incoming), (0x1a02, &parts)] {
            let actual = directories
                .get(tag)
                .ok_or("Falta una carpeta privada eD2k")?
                .text();
            if std::fs::canonicalize(actual).map_err(|e| e.to_string())? != *expected {
                return Err(
                    "El motor eD2k no aceptó las carpetas privadas. No se ha iniciado la descarga"
                        .into(),
                );
            }
        }
        // Restart never resumes files silently, including a previous interrupted exit.
        let (_, queue) = client.request(0xd, vec![]).await?;
        for row in queue.iter().filter(|t| t.id == 0x300) {
            if let Some(h) = row.get(0x31e) {
                let _ = client
                    .request(0x19, vec![Tag::hash(0x300, h.data.clone())])
                    .await;
            }
        }
        for (address, name) in [
            ("176.125.242.230:5555", "eNode"),
            ("85.17.116.222:6082", "eD2k"),
            ("77.42.68.79:4232", "Nordic"),
        ] {
            let _ = client
                .request(
                    0x31,
                    vec![Tag::string(0x503, address), Tag::string(0x501, name)],
                )
                .await;
        }
        let tags = self
            .shared_paths
            .lock()
            .unwrap()
            .iter()
            .map(|p| {
                let mut t = Tag::string(0x2000, &ini_path(p));
                t.children.push(Tag::number(0x2001, 1));
                t
            })
            .collect();
        let (_, reply) = client.request(0x5e, tags).await?;
        if reply.iter().any(|t| t.id == 0x2002) {
            return Err("aMule ha rechazado una carpeta compartida".into());
        }
        *running = Some(Running {
            child,
            _job: job,
            client,
        });
        Ok(())
    }
    fn watch(self: &Arc<Self>) {
        let mut monitor = self.monitor.lock().unwrap();
        if monitor.as_ref().is_some_and(|j| !j.is_finished()) {
            return;
        }
        let weak = Arc::downgrade(self);
        *monitor = Some(tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let Some(this) = weak.upgrade() else { break };
                let _guard = this.control.lock().await;
                if let Err(e) = this.update().await {
                    {
                        let mut store = this.store.lock().unwrap();
                        for r in store.records.values_mut() {
                            if !matches!(r.state.as_str(), "paused" | "cancelled" | "completed") {
                                r.state = "error".into();
                                r.error = Some(e.clone());
                                r.download_speed = 0;
                            }
                        }
                    }
                    if let Some(engine) = this.running.lock().await.take() {
                        engine.stop().await;
                    }
                    let _ = this.save();
                }
            }
        }));
    }
    pub async fn add(
        self: &Arc<Self>,
        uri: String,
        name: String,
        folder: PathBuf,
    ) -> Result<String, String> {
        let uri = crate::ed2k::validate_link(&uri)?;
        let p: Vec<_> = uri.split('|').collect();
        let size: u64 = p[3].parse().map_err(|_| "Tamaño inválido")?;
        let hash = p[4].to_lowercase();
        let id = format!("ed2k:{hash}:{size}");
        let _guard = self.control.lock().await;
        if self.store.lock().unwrap().records.contains_key(&id) {
            return Ok(id);
        }
        let selected = safe_dir(&folder)?;
        let destination = selected;
        if destination.join(filename(&name)).exists() {
            return Err("Ya existe un archivo con ese nombre; no se sobrescribe. Cambia su nombre o elige otra carpeta".into());
        }
        if self.store.lock().unwrap().root.is_none() {
            let root = safe_dir(&self.data.join("workspace"))?;
            if std::fs::read_dir(&root)
                .map_err(|e| e.to_string())?
                .next()
                .is_some()
            {
                return Err("La carpeta temporal eD2k ya contiene archivos de otro perfil. Elige otra carpeta".into());
            }
            self.store.lock().unwrap().root = Some(root);
            self.save()?;
        }
        self.start().await?;
        // Do not trust search filenames as paths, nor accept source/server commands.
        let safe_uri = crate::ed2k::link(&format!("{hash}-{}", filename(&name)), size, &hash);
        {
            let mut running = self.running.lock().await;
            let client = &mut running.as_mut().unwrap().client;
            client.request(9, vec![Tag::string(0, &safe_uri)]).await?;
            client
                .request(0x1a, vec![Tag::hash(0x300, hex::decode(&hash).unwrap())])
                .await?;
            let _ = client.request(0x4a, vec![]).await;
        }
        self.store.lock().unwrap().records.insert(
            id.clone(),
            Download {
                id: id.clone(),
                name: filename(&name),
                folder: destination.to_string_lossy().into_owned(),
                state: "waiting".into(),
                completed_bytes: 0,
                total_bytes: size,
                download_speed: 0,
                upload_speed: 0,
                error: None,
                uri: safe_uri,
            },
        );
        self.save()?;
        self.watch();
        Ok(id)
    }
    pub async fn remove(self: &Arc<Self>, id: &str, delete_files: bool) -> Result<(), String> {
        let _guard = self.control.lock().await;
        let record = self
            .store
            .lock()
            .unwrap()
            .records
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        if record.state != "completed" {
            self.start().await?;
            let hash = id.split(':').nth(1).ok_or("Hash inválido")?;
            let mut running = self.running.lock().await;
            // Remove the managed partial from the engine queue, stopping future writes.
            // Keeping files means stopping instead, so its private partial stays recoverable.
            running
                .as_mut()
                .unwrap()
                .client
                .request(
                    if delete_files { 0x1d } else { 0x1b },
                    vec![Tag::hash(
                        0x300,
                        hex::decode(hash).map_err(|e| e.to_string())?,
                    )],
                )
                .await?;
        }
        if delete_files {
            crate::downloads::delete_payload(
                Path::new(&record.folder),
                &[PathBuf::from(filename(&record.name))],
            )?;
        }
        self.store.lock().unwrap().records.remove(id);
        self.save()?;
        Ok(())
    }
    pub async fn action(self: &Arc<Self>, id: &str, action: &str) -> Result<(), String> {
        let _guard = self.control.lock().await;
        if !matches!(action, "pause" | "resume" | "cancel") {
            return Err("Acción desconocida".into());
        }
        let old = self
            .store
            .lock()
            .unwrap()
            .records
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        if old.state == "completed" {
            return Err("La descarga ya ha terminado".into());
        }
        self.start().await?;
        let hash = id.split(':').nth(1).ok_or("Hash inválido")?;
        {
            let mut running = self.running.lock().await;
            let c = &mut running.as_mut().unwrap().client;
            c.request(
                match action {
                    "pause" => 0x19,
                    "resume" => 0x1a,
                    _ => 0x1b,
                },
                vec![Tag::hash(
                    0x300,
                    hex::decode(hash).map_err(|_| "Hash inválido")?,
                )],
            )
            .await?;
            if action == "resume" {
                let _ = c.request(0x4a, vec![]).await;
            }
        }
        if let Some(r) = self.store.lock().unwrap().records.get_mut(id) {
            r.state = match action {
                "resume" => "waiting",
                "pause" => "paused",
                _ => "cancelled",
            }
            .into();
            r.error = None;
            r.download_speed = 0;
        }
        self.save()?;
        if self.idle() {
            if let Some(engine) = self.running.lock().await.take() {
                engine.stop().await;
            }
        }
        self.watch();
        Ok(())
    }
    async fn update(&self) -> Result<(), String> {
        let mut running = self.running.lock().await;
        let Some(engine) = running.as_mut() else {
            return Ok(());
        };
        if self
            .stop_search
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            let _ = engine.client.request(0x27, vec![]).await;
        }
        let (_, queue) = engine.client.request(0xd, vec![]).await?;
        let (_, uploads) = engine.client.request(0xe, vec![]).await?;
        let mut upload_rates = BTreeMap::<u64, u64>::new();
        for peer in uploads.iter().filter(|t| t.id == 0x600) {
            *upload_rates.entry(peer.value(0x61f)).or_default() += peer.value(0x60d);
        }
        let mut completed = vec![];
        for row in queue.iter().filter(|t| t.id == 0x300) {
            let Some(hash) = row.get(0x31e).map(|h| hex::encode(&h.data)) else {
                continue;
            };
            let size = row.value(0x303);
            let id = format!("ed2k:{hash}:{size}");
            let mut store = self.store.lock().unwrap();
            let Some(r) = store.records.get_mut(&id) else {
                continue;
            };
            if r.state == "completed" {
                continue;
            }
            r.completed_bytes = row.value(0x306).min(r.total_bytes);
            r.download_speed = row.value(0x307);
            r.upload_speed = upload_rates.get(&row.num()).copied().unwrap_or(0);
            r.error = None;
            if row.value(0x308) == 9 {
                completed.push((id.clone(), row.clone()));
                r.state = "finalizing".into();
            } else if !matches!(r.state.as_str(), "paused" | "cancelled") {
                r.state = match row.value(0x308) {
                    2 | 3 | 10 => "checking",
                    4 | 5 => "error",
                    7 => "paused",
                    8 => "finalizing",
                    _ => {
                        if r.download_speed > 0 {
                            "downloading"
                        } else {
                            "waiting"
                        }
                    }
                }
                .into();
                if r.state == "error" {
                    r.error = Some(
                        "El motor no puede continuar; comprueba el espacio disponible y reanuda"
                            .into(),
                    )
                }
            }
        }
        // Only the private incoming directory is considered, never user shares.
        // Finalization verifies the complete eD2k hash before publishing a file.
        let pending: Vec<_> = self
            .store
            .lock()
            .unwrap()
            .records
            .values()
            .filter(|r| r.state != "completed")
            .cloned()
            .collect();
        let incoming = self
            .store
            .lock()
            .unwrap()
            .root
            .clone()
            .unwrap()
            .join("incoming");
        for record in pending {
            if completed.iter().any(|(id, _)| id == &record.id) {
                continue;
            }
            let hash = record.id.split(':').nth(1).ok_or("Hash eD2k inválido")?;
            for name in [
                format!("{hash}-{}", filename(&record.name)),
                record.name.clone(),
            ] {
                let path = incoming.join(&name);
                if path.is_file() && path.metadata().is_ok_and(|m| m.len() == record.total_bytes) {
                    let row = Tag {
                        id: 0x400,
                        kind: 1,
                        data: vec![],
                        children: vec![
                            Tag::string(0x301, &name),
                            Tag::string(0x408, &incoming.to_string_lossy()),
                        ],
                    };
                    completed.push((record.id.clone(), row));
                    break;
                }
            }
        }
        for (id, row) in completed {
            let record = self
                .store
                .lock()
                .unwrap()
                .records
                .get(&id)
                .cloned()
                .unwrap();
            let root = self.store.lock().unwrap().root.clone().unwrap();
            let path = row.get(0x408).map(Tag::text).unwrap_or_default();
            let mut source = if Path::new(&path).is_absolute() {
                PathBuf::from(path)
            } else {
                root.join("incoming").join(
                    row.get(0x301)
                        .map(Tag::text)
                        .unwrap_or_else(|| record.name.clone()),
                )
            };
            if source.is_dir() {
                let name = row
                    .get(0x301)
                    .map(Tag::text)
                    .ok_or("Falta el nombre del archivo eD2k")?;
                if Path::new(&name)
                    .file_name()
                    .is_none_or(|n| n != name.as_str())
                {
                    return Err("Nombre de archivo eD2k inválido".into());
                }
                source = source.join(name);
            }
            let target = PathBuf::from(&record.folder).join(filename(&record.name));
            let hash = id.split(':').nth(1).unwrap().to_string();
            let result = tokio::task::spawn_blocking(move || {
                finalize(&root, &source, &target, record.total_bytes, &hash)
            })
            .await
            .map_err(|e| e.to_string())?;
            let success = result.is_ok();
            {
                let mut store = self.store.lock().unwrap();
                let r = store.records.get_mut(&id).unwrap();
                r.download_speed = 0;
                r.upload_speed = 0;
                match result {
                    Ok(()) => {
                        r.state = "completed".into();
                        r.completed_bytes = r.total_bytes;
                        r.error = None
                    }
                    Err(e) => {
                        r.state = "error".into();
                        r.error = Some(e)
                    }
                }
            }
            // Moving the file out and reloading releases it and removes it from sharing.
            let _ = engine.client.request(0x23, vec![]).await;
            if !success {
                let _ = engine.client.request(0x4b, vec![]).await;
            }
        }
        self.save()?;
        if self.idle() {
            if let Some(engine) = running.take() {
                engine.stop().await;
            }
        }
        Ok(())
    }
    fn idle(&self) -> bool {
        if !self.shared_paths.lock().unwrap().is_empty() {
            return false;
        }
        if self.kad_users.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            return false;
        }
        self.store.lock().unwrap().records.values().all(|r| {
            matches!(
                r.state.as_str(),
                "paused" | "cancelled" | "completed" | "error"
            )
        })
    }
    pub async fn set_shared_folders(self: &Arc<Self>, paths: Vec<PathBuf>) -> Result<(), String> {
        let _guard = self.control.lock().await;
        if *self.shared_paths.lock().unwrap() == paths {
            return Ok(());
        }
        for path in &paths {
            crate::sharing::validate_tree(path)?;
        }
        let active: Vec<_> = self
            .store
            .lock()
            .unwrap()
            .records
            .values()
            .filter(|r| {
                matches!(
                    r.state.as_str(),
                    "waiting" | "resolving" | "checking" | "downloading"
                )
            })
            .map(|r| r.id.clone())
            .collect();
        if let Some(engine) = self.running.lock().await.take() {
            engine.stop().await;
        }
        *self.shared_paths.lock().unwrap() = paths;
        if !self.shared_paths.lock().unwrap().is_empty() || !active.is_empty() {
            if self.store.lock().unwrap().root.is_none() {
                let root = safe_dir(&self.data.join("workspace"))?;
                self.store.lock().unwrap().root = Some(root);
                self.save()?;
            }
            self.start().await?;
            let mut running = self.running.lock().await;
            let c = &mut running.as_mut().unwrap().client;
            for id in active {
                if let Some(hash) = id.split(':').nth(1) {
                    c.request(
                        0x1a,
                        vec![Tag::hash(
                            0x300,
                            hex::decode(hash).map_err(|_| "Hash inválido")?,
                        )],
                    )
                    .await?;
                }
            }
            c.request(0x4a, vec![]).await?;
            self.watch();
        }
        Ok(())
    }
    pub async fn set_limits(&self, down: u32, up: u32) -> Result<(), String> {
        if *self.limits.lock().unwrap() == (down, up) {
            return Ok(());
        }
        let mut running = self.running.lock().await;
        if let Some(engine) = running.as_mut() {
            engine
                .client
                .request(
                    0x40,
                    vec![Tag {
                        id: 0x1300,
                        kind: 1,
                        data: vec![],
                        children: vec![
                            Tag::number(0x1303, down.into()),
                            Tag::number(0x1304, up.into()),
                        ],
                    }],
                )
                .await?;
        }
        *self.limits.lock().unwrap() = (down, up);
        Ok(())
    }
    pub async fn shutdown(&self) {
        if let Some(job) = self.monitor.lock().unwrap().take() {
            job.abort()
        }
        let _guard = self.control.lock().await;
        let mut running = self.running.lock().await;
        if let Some(mut engine) = running.take() {
            if let Ok((_, queue)) = engine.client.request(0xd, vec![]).await {
                for row in queue.iter().filter(|t| t.id == 0x300) {
                    if let Some(h) = row.get(0x31e) {
                        let _ = engine
                            .client
                            .request(0x19, vec![Tag::hash(0x300, h.data.clone())])
                            .await;
                    }
                }
            }
            engine.stop().await;
        }
        for r in self.store.lock().unwrap().records.values_mut() {
            if !matches!(r.state.as_str(), "completed" | "cancelled") {
                r.state = "paused".into()
            }
            r.download_speed = 0;
            r.upload_speed = 0;
        }
        let _ = self.save();
    }
}
fn finalize(
    root: &Path,
    source: &Path,
    target: &Path,
    size: u64,
    hash: &str,
) -> Result<(), String> {
    let incoming = safe_dir(&root.join("incoming"))?;
    let canonical = std::fs::canonicalize(source).map_err(|e| e.to_string())?;
    if !canonical.starts_with(&incoming)
        || std::fs::symlink_metadata(source)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("El archivo terminado está fuera de la carpeta privada".into());
    }
    let parent = target.parent().ok_or("Destino inválido")?;
    if safe_dir(parent)? != parent {
        return Err("La carpeta de destino ha cambiado".into());
    }
    verify_file(&canonical, size, hash)?;
    // Verify a temporary copy before publishing it, without replacing any file.
    if target.exists() {
        verify_file(target, size, hash)?;
    } else {
        use std::io::Write;
        let mut input = std::fs::File::open(&canonical).map_err(|e| e.to_string())?;
        let mut output = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        output.as_file().sync_all().map_err(|e| e.to_string())?;
        verify_file(output.path(), size, hash)?;
        let persisted = output
            .persist_noclobber(target)
            .map_err(|e| e.to_string())?;
        drop(persisted);
    }
    std::fs::remove_file(&canonical).map_err(|e| {
        format!("El archivo está guardado pero no se pudo liberar el temporal: {e}")
    })?;
    Ok(())
}
fn verify_file(path: &Path, size: u64, expected: &str) -> Result<(), String> {
    use md4::Md4;
    use std::io::Read;
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("El archivo no puede ser un enlace".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err("El archivo no puede ser un enlace".into());
        }
    }
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if f.metadata().map_err(|e| e.to_string())?.len() != size {
        return Err("El tamaño del archivo no coincide".into());
    }
    let mut remaining = size;
    let mut hashes = vec![];
    let mut buffer = vec![0; 64 * 1024];
    while remaining > 0 {
        let mut part = Md4::new();
        let mut left = remaining.min(9_728_000);
        remaining -= left;
        while left > 0 {
            let n = (left as usize).min(buffer.len());
            f.read_exact(&mut buffer[..n]).map_err(|e| e.to_string())?;
            part.update(&buffer[..n]);
            left -= n as u64;
        }
        hashes.extend_from_slice(&part.finalize());
    }
    // eMule hashes include an empty terminal part at exact part-size multiples.
    if size % 9_728_000 == 0 {
        hashes.extend_from_slice(&Md4::digest([]));
    }
    let actual = if hashes.len() == 16 {
        hex::encode(hashes)
    } else {
        hex::encode(Md4::digest(hashes))
    };
    if actual != expected.to_lowercase() {
        return Err("La verificación eD2k del archivo ha fallado".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    static ENGINE_TEST: AsyncMutex<()> = AsyncMutex::const_new(());
    // Isolate only the test fixture mutex; shipped engine bytes remain official.
    #[cfg(not(windows))]
    fn isolated_engine(_root: &Path) -> PathBuf {
        crate::platform::dev_amule()
    }
    #[cfg(windows)]
    fn isolated_engine(root: &Path) -> PathBuf {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../vendor/amule-3.1.0/amule-portable-x64/bin/amuled.exe");
        let folder = root.join("isolated-engine");
        std::fs::create_dir_all(&folder).unwrap();
        for entry in std::fs::read_dir(source.parent().unwrap()).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().is_some_and(|e| e == "dll") {
                std::fs::copy(&p, folder.join(p.file_name().unwrap())).unwrap();
            }
        }
        let mut bytes = std::fs::read(source).unwrap();
        let mut found = 0;
        for needle in [
            b"muleLock".to_vec(),
            "muleLock"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect(),
        ] {
            for at in 0..=bytes.len() - needle.len() {
                if bytes[at..at + needle.len()] == needle {
                    bytes[at] = b'z';
                    found += 1;
                }
            }
        }
        assert!(found > 0);
        let target = folder.join("amuled.exe");
        std::fs::write(&target, bytes).unwrap();
        target
    }
    #[tokio::test]
    #[ignore = "Prueba de bootstrap y búsqueda en la red Kad pública"]
    async fn live_kad() {
        let _lock = ENGINE_TEST.lock().await;
        let root = sandbox();
        let engine = Downloads::new(isolated_engine(&root), root.join("config")).unwrap();
        let result = engine
            .search_kad(
                "ubuntu",
                Arc::new(|rows| {
                    println!("Kad: {} resultados con fuentes completas", rows.len());
                }),
            )
            .await;
        engine.shutdown().await;
        println!("Kad: {result:?}");
        assert!(result.is_ok(), "{result:?}");
    }

    async fn test_server(
        ip: std::net::Ipv4Addr,
        source: Arc<Mutex<Option<(Vec<u8>, u16)>>>,
    ) -> (u16, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind((ip, 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let job = tokio::spawn(async move {
            let mut client_id = 0x281e140au32;
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                client_id += 0x01000000;
                let id = client_id;
                let source = source.clone();
                tokio::spawn(async move {
                    loop {
                        if socket.read_u8().await.ok() != Some(0xe3) {
                            break;
                        }
                        let Ok(len) = socket.read_u32_le().await else {
                            break;
                        };
                        if !(1..=1048576).contains(&len) {
                            break;
                        }
                        let mut payload = vec![0; len as usize];
                        if socket.read_exact(&mut payload).await.is_err() {
                            break;
                        }
                        let response = match payload[0] {
                            1 => {
                                let mut p = vec![0x40];
                                p.extend_from_slice(&id.to_le_bytes());
                                p.extend_from_slice(&0x119u32.to_le_bytes());
                                Some(p)
                            }
                            0x19 | 0x23 => {
                                let seed = source.lock().unwrap().clone();
                                seed.and_then(|(hash, seed_port)| {
                                    if payload.len() < 17 || payload[1..17] != hash {
                                        return None;
                                    }
                                    let mut p = vec![0x42];
                                    p.extend(hash);
                                    p.push(1);
                                    p.extend(ip.octets());
                                    p.extend_from_slice(&seed_port.to_le_bytes());
                                    Some(p)
                                })
                            }
                            _ => None,
                        };
                        if let Some(response) = response {
                            if socket.write_u8(0xe3).await.is_err()
                                || socket.write_u32_le(response.len() as u32).await.is_err()
                                || socket.write_all(&response).await.is_err()
                            {
                                break;
                            }
                        }
                    }
                });
            }
        });
        (port, job)
    }
    fn server_met(data: &Path, ip: std::net::Ipv4Addr, port: u16) {
        let mut met = vec![0xe0];
        met.extend_from_slice(&1u32.to_le_bytes());
        met.extend(ip.octets());
        met.extend_from_slice(&port.to_le_bytes());
        met.extend_from_slice(&0u32.to_le_bytes());
        std::fs::write(data.join("server.met"), met).unwrap();
    }
    fn sandbox() -> PathBuf {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/ed2k-download-tests")
            .join(format!("{:016x}", rand::random::<u64>()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn verifies_and_releases_completed_file() {
        let root = sandbox();
        let incoming = safe_dir(&root.join("incoming")).unwrap();
        let destination = safe_dir(&root.join("output")).unwrap();
        let payload = b"MAGNETRON eD2k test file";
        let hash = hex::encode(md4::Md4::digest(payload));
        let source = incoming.join("file.txt");
        let target = destination.join("file.txt");
        std::fs::write(&source, payload).unwrap();
        assert!(finalize(
            &root,
            &source,
            &target,
            payload.len() as u64,
            &"0".repeat(32)
        )
        .is_err());
        assert!(source.exists());
        finalize(&root, &source, &target, payload.len() as u64, &hash).unwrap();
        assert!(!source.exists());
        assert_eq!(std::fs::read(&target).unwrap(), payload);
        std::fs::remove_file(&target).unwrap();
        assert_eq!(filename("../CON.txt"), "_CON.txt");
        assert!(filename("CON.txt").starts_with("archivo_"));
    }
    #[tokio::test]
    async fn bundled_engine_add_pause_stop_resume_restart() {
        let _lock = ENGINE_TEST.lock().await;
        let root = sandbox();
        let exe = isolated_engine(&root);
        let engine = Downloads::new(exe.clone(), root.join("config")).unwrap();
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
        socket.connect("8.8.8.8:80").unwrap();
        let std::net::IpAddr::V4(ip) = socket.local_addr().unwrap().ip() else {
            panic!("IPv4 required")
        };
        let (port, server_job) = test_server(ip, Arc::new(Mutex::new(None))).await;
        server_met(&engine.data, ip, port);
        let uri = crate::ed2k::link("test.bin", 12345, &"a".repeat(32));
        let result = engine.add(uri, "test.bin".into(), root.clone()).await;
        if result.is_err() {
            engine.shutdown().await;
        }
        let id = result.unwrap();
        engine.action(&id, "pause").await.unwrap();
        assert_eq!(engine.list()[0].state, "paused");
        engine.action(&id, "cancel").await.unwrap();
        assert_eq!(engine.list()[0].state, "cancelled");
        engine.action(&id, "resume").await.unwrap();
        engine.update().await.unwrap();
        assert!(matches!(
            engine.list()[0].state.as_str(),
            "waiting" | "downloading"
        ));
        engine.shutdown().await;
        drop(engine);
        let restored = Downloads::new(exe, root.join("config")).unwrap();
        assert_eq!(restored.list()[0].state, "paused");
        restored.action(&id, "resume").await.unwrap();
        restored.action(&id, "pause").await.unwrap();
        restored.remove(&id, true).await.unwrap();
        assert!(restored.list().is_empty());
        restored.shutdown().await;
        server_job.abort();
    }
    #[tokio::test]
    async fn real_ed2k_transfer_and_completed_file_is_unlocked() {
        let _lock = ENGINE_TEST.lock().await;
        let root = sandbox();
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
        socket.connect("8.8.8.8:80").unwrap();
        let std::net::IpAddr::V4(ip) = socket.local_addr().unwrap().ip() else {
            panic!("IPv4 required")
        };
        let source = Arc::new(Mutex::new(None));
        let (server_port, server_job) = test_server(ip, source.clone()).await;
        let exe = isolated_engine(&root);
        // wxWidgets uses a global Windows mutex regardless of --config-dir.
        // Test clients use distinct fixture mutexes; the distributed engine remains official.
        #[cfg(windows)]
        let seed_exe = {
            let fixture = root.join("test-seeder");
            std::fs::create_dir_all(&fixture).unwrap();
            for entry in std::fs::read_dir(exe.parent().unwrap()).unwrap() {
                let p = entry.unwrap().path();
                if p.extension().is_some_and(|e| e == "dll") {
                    std::fs::hard_link(&p, fixture.join(p.file_name().unwrap())).unwrap();
                }
            }
            let mut seed_binary = std::fs::read(&exe).unwrap();
            let mut replacements = 0;
            for needle in [
                b"zuleLock".to_vec(),
                "zuleLock"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<_>>(),
            ] {
                let mut at = 0;
                while at + needle.len() <= seed_binary.len() {
                    if seed_binary[at..at + needle.len()] == needle {
                        seed_binary[at] = b't';
                        replacements += 1;
                        at += needle.len()
                    } else {
                        at += 1
                    }
                }
            }
            assert!(replacements > 0);
            let seed_exe = fixture.join("test-seeder.exe");
            std::fs::write(&seed_exe, seed_binary).unwrap();
            seed_exe
        };
        #[cfg(not(windows))]
        let seed_exe = exe.clone();
        let seed = Downloads::new(seed_exe, root.join("seed-config")).unwrap();
        server_met(&seed.data, ip, server_port);
        let seed_root = safe_dir(&root.join("seed-private")).unwrap();
        let incoming = safe_dir(&seed_root.join("incoming")).unwrap();
        let payload: Vec<u8> = (0..131072)
            .map(|i| ((i * 17 + i / 200) % 251) as u8)
            .collect();
        let hash = hex::encode(md4::Md4::digest(&payload));
        std::fs::write(incoming.join("MAGNETRON-test.bin"), &payload).unwrap();
        seed.store.lock().unwrap().root = Some(seed_root);
        seed.start().await.unwrap();
        let client = Downloads::new(exe, root.join("client-config")).unwrap();
        server_met(&client.data, ip, server_port);
        let private = safe_dir(&root.join("client-private")).unwrap();
        client.store.lock().unwrap().root = Some(private);
        client.start().await.unwrap();
        let result = async {
            tokio::time::timeout(Duration::from_secs(30), async {
                loop {
                    let (_, rows) = seed
                        .running
                        .lock()
                        .await
                        .as_mut()
                        .unwrap()
                        .client
                        .request(0x10, vec![])
                        .await
                        .unwrap();
                    if rows
                        .iter()
                        .any(|t| t.get(0x31e).is_some_and(|h| hex::encode(&h.data) == hash))
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            })
            .await
            .map_err(|_| "Seeder did not hash the synthetic file".to_string())?;
            let conf = std::fs::read_to_string(root.join("seed-config/amule.conf")).unwrap();
            let port = conf
                .lines()
                .find_map(|l| l.strip_prefix("Port="))
                .unwrap()
                .trim();
            *source.lock().unwrap() = Some((hex::decode(&hash).unwrap(), port.parse().unwrap()));
            seed.running
                .lock()
                .await
                .as_mut()
                .unwrap()
                .client
                .request(0x4a, vec![])
                .await?;
            let uri = crate::ed2k::link("MAGNETRON-test.bin", payload.len() as u64, &hash);
            let id = client
                .add(uri.clone(), "MAGNETRON-test.bin".into(), root.clone())
                .await?;
            assert_eq!(client.folder(&id)?, root.canonicalize().unwrap());
            assert!(!root.join("MAGNETRON").exists());
            assert!(!root.join("ed2k").exists());
            tokio::time::timeout(Duration::from_secs(100), async {
                loop {
                    {
                        let _guard = client.control.lock().await;
                        client.update().await?;
                    }
                    let row = client.list().into_iter().find(|r| r.id == id).unwrap();
                    if row.state == "completed" {
                        let file = PathBuf::from(row.folder).join("MAGNETRON-test.bin");
                        assert_eq!(
                            file,
                            root.canonicalize().unwrap().join("MAGNETRON-test.bin")
                        );
                        assert_eq!(std::fs::read(&file).unwrap(), payload);
                        #[cfg(windows)]
                        {
                            use std::os::windows::fs::OpenOptionsExt;
                            assert!(std::fs::OpenOptions::new()
                                .read(true)
                                .share_mode(0)
                                .open(&file)
                                .is_ok());
                        }
                        client.remove(&id, true).await?;
                        assert!(!file.exists());
                        assert!(root.is_dir());
                        assert!(client.list().is_empty());
                        return Ok::<(), String>(());
                    }
                    if row.state == "error" {
                        return Err(row.error.unwrap_or_default());
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            })
            .await
            .map_err(|_| "Timed out downloading from the controlled eD2k seed".to_string())?
        }
        .await;
        client.shutdown().await;
        seed.shutdown().await;
        server_job.abort();
        assert!(result.is_ok(), "{result:?}; logs in {}", root.display());
    }
    #[tokio::test]
    async fn arbitrary_ed2k_folder_can_be_shared_and_removed() {
        let _lock = ENGINE_TEST.lock().await;
        let root = sandbox();
        let engine = Downloads::new(isolated_engine(&root), root.join("config")).unwrap();
        let share = root.join("user-files");
        std::fs::create_dir(&share).unwrap();
        let payload = vec![31u8; 65536];
        let file = share.join("share-test.bin");
        std::fs::write(&file, &payload).unwrap();
        let hash = hex::encode(md4::Md4::digest(&payload));
        let result = async {
            engine.set_shared_folders(vec![share.clone()]).await?;
            tokio::time::timeout(Duration::from_secs(30), async {
                loop {
                    let (_, rows) = engine
                        .running
                        .lock()
                        .await
                        .as_mut()
                        .unwrap()
                        .client
                        .request(0x10, vec![])
                        .await?;
                    if rows
                        .iter()
                        .any(|t| t.get(0x31e).is_some_and(|h| hex::encode(&h.data) == hash))
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
                Ok::<(), String>(())
            })
            .await
            .map_err(|_| "No se publicó la carpeta elegida")??;
            assert_eq!(std::fs::read(&file).unwrap(), payload);
            let removed = std::fs::remove_file(&file);
            println!("Eliminar archivo compartido eD2k: {removed:?}");
            engine.set_shared_folders(vec![]).await?;
            if file.exists() {
                std::fs::remove_file(&file).map_err(|e| e.to_string())?;
            }
            Ok::<(), String>(())
        }
        .await;
        engine.shutdown().await;
        assert!(result.is_ok(), "{result:?}");
    }
}
