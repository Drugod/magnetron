//! User-selected shares. Seeding existing folders always uses read-only storage.
use librqbit::{
    storage::{BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage},
    ManagedTorrentShared, TorrentMetadata,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Folder {
    pub path: String,
    pub bittorrent: bool,
    pub ed2k: bool,
}
#[derive(Clone, Serialize)]
pub struct Status {
    pub path: String,
    pub state: String,
    pub magnet: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone)]
struct ReadOnlyFactory {
    root: PathBuf,
}
struct ReadOnlyStorage {
    root: PathBuf,
    files: Mutex<Vec<std::fs::File>>,
}
impl StorageFactory for ReadOnlyFactory {
    type Storage = ReadOnlyStorage;
    fn create(
        &self,
        _: &ManagedTorrentShared,
        _: &TorrentMetadata,
    ) -> anyhow::Result<Self::Storage> {
        Ok(ReadOnlyStorage {
            root: self.root.clone(),
            files: Mutex::new(vec![]),
        })
    }
    fn clone_box(&self) -> BoxStorageFactory {
        self.clone().boxed()
    }
}
impl TorrentStorage for ReadOnlyStorage {
    fn init(&mut self, _: &ManagedTorrentShared, meta: &TorrentMetadata) -> anyhow::Result<()> {
        let mut files = vec![];
        for info in meta.info.iter_file_details() {
            let relative = info.filename.to_pathbuf();
            anyhow::ensure!(
                relative
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
                "Ruta compartida inválida"
            );
            let mut path = self.root.clone();
            for component in relative.components() {
                path.push(component);
                crate::downloads::reject_links(&path).map_err(anyhow::Error::msg)?;
            }
            anyhow::ensure!(
                path.canonicalize()?.starts_with(&self.root),
                "Ruta fuera de la carpeta compartida"
            );
            let file = std::fs::OpenOptions::new().read(true).open(path)?;
            anyhow::ensure!(
                file.metadata()?.len() == info.len,
                "El archivo cambió durante la preparación"
            );
            files.push(file);
        }
        *self.files.lock().unwrap() = files;
        Ok(())
    }
    fn pread_exact(&self, id: usize, offset: u64, buffer: &mut [u8]) -> anyhow::Result<()> {
        use std::io::{Read, Seek, SeekFrom};
        let mut files = self.files.lock().unwrap();
        let f = files
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Archivo no disponible"))?;
        f.seek(SeekFrom::Start(offset))?;
        f.read_exact(buffer)?;
        Ok(())
    }
    fn pwrite_all(&self, _: usize, _: u64, _: &[u8]) -> anyhow::Result<()> {
        anyhow::bail!("Compartición de solo lectura")
    }
    fn remove_file(&self, _: usize, _: &Path) -> anyhow::Result<()> {
        anyhow::bail!("No se borran archivos compartidos")
    }
    fn remove_directory_if_empty(&self, _: &Path) -> anyhow::Result<()> {
        anyhow::bail!("No se borran carpetas compartidas")
    }
    fn ensure_file_length(&self, id: usize, length: u64) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.files
                .lock()
                .unwrap()
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("Archivo no disponible"))?
                .metadata()?
                .len()
                == length,
            "El archivo cambió de tamaño"
        );
        Ok(())
    }
    fn take(&self) -> anyhow::Result<Box<dyn TorrentStorage>> {
        Ok(Box::new(Self {
            root: self.root.clone(),
            files: Mutex::new(std::mem::take(&mut *self.files.lock().unwrap())),
        }))
    }
}
pub fn validate_tree(path: &Path) -> Result<(), String> {
    fn walk(path: &Path, depth: usize, count: &mut usize) -> Result<(), String> {
        *count += 1;
        if depth > 64 || *count > 100000 {
            return Err("Carpeta demasiado grande: máximo 100000 entradas y 64 niveles".into());
        }
        let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("No se comparten enlaces o uniones".into());
            }
        }
        if meta.file_type().is_symlink() {
            return Err("No se comparten enlaces".into());
        }
        if meta.is_dir() {
            for item in std::fs::read_dir(path).map_err(|e| e.to_string())? {
                walk(&item.map_err(|e| e.to_string())?.path(), depth + 1, count)?;
            }
        }
        Ok(())
    }
    walk(path, 0, &mut 0)
}
pub struct Sharing {
    bt: Arc<crate::downloads::Downloads>,
    ed: Arc<crate::ed2k_downloads::Downloads>,
    desired: Mutex<Vec<Folder>>,
    statuses: Mutex<BTreeMap<String, Status>>,
    handles: tokio::sync::Mutex<BTreeMap<String, Arc<librqbit::ManagedTorrent>>>,
    generation: AtomicU64,
    job: Mutex<Option<tokio::task::JoinHandle<()>>>,
    control: tokio::sync::Mutex<()>,
}
impl Sharing {
    pub fn new(
        bt: Arc<crate::downloads::Downloads>,
        ed: Arc<crate::ed2k_downloads::Downloads>,
    ) -> Arc<Self> {
        Arc::new(Self {
            bt,
            ed,
            desired: Mutex::new(vec![]),
            statuses: Mutex::new(BTreeMap::new()),
            handles: tokio::sync::Mutex::new(BTreeMap::new()),
            generation: AtomicU64::new(0),
            job: Mutex::new(None),
            control: tokio::sync::Mutex::new(()),
        })
    }
    pub fn list(&self) -> Vec<Status> {
        self.statuses.lock().unwrap().values().cloned().collect()
    }
    pub fn bt_active(&self) -> bool {
        self.desired.lock().unwrap().iter().any(|f| f.bittorrent)
    }
    pub fn ed_active(&self) -> bool {
        self.desired.lock().unwrap().iter().any(|f| f.ed2k)
    }
    pub async fn configure(
        self: &Arc<Self>,
        folders: Vec<Folder>,
        force: bool,
    ) -> Result<(), String> {
        let _control = self.control.lock().await;
        if !force && *self.desired.lock().unwrap() == folders {
            return Ok(());
        }
        let checked = folders.clone();
        tokio::task::spawn_blocking(move || {
            let mut roots: Vec<PathBuf> = vec![];
            for f in checked.iter().filter(|f| f.bittorrent || f.ed2k) {
                if !Path::new(&f.path).is_dir() {
                    return Err("Una carpeta compartida ya no está disponible".to_string());
                }
                crate::downloads::reject_links(Path::new(&f.path))?;
                validate_tree(Path::new(&f.path))?;
                let root = Path::new(&f.path)
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if roots
                    .iter()
                    .any(|r| root.starts_with(r) || r.starts_with(&root))
                {
                    return Err("Elige carpetas compartidas que no se solapen".into());
                }
                roots.push(root);
            }
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| e.to_string())??;
        self.ed
            .set_shared_folders(
                folders
                    .iter()
                    .filter(|f| f.ed2k)
                    .map(|f| PathBuf::from(&f.path))
                    .collect(),
            )
            .await?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(job) = self.job.lock().unwrap().take() {
            job.abort()
        }
        let mut handles = self.handles.lock().await;
        if !handles.is_empty() {
            let session = self.bt.session().await?;
            for (_, h) in std::mem::take(&mut *handles) {
                session
                    .delete(librqbit::api::TorrentIdOrHash::Id(h.id()), false)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        drop(handles);
        *self.desired.lock().unwrap() = folders.clone();
        {
            let mut statuses = self.statuses.lock().unwrap();
            statuses.clear();
            for f in &folders {
                statuses.insert(
                    f.path.clone(),
                    Status {
                        path: f.path.clone(),
                        state: if f.bittorrent {
                            "Preparando torrent"
                        } else if f.ed2k {
                            "Compartiendo en eD2k"
                        } else {
                            "Sin compartir"
                        }
                        .into(),
                        magnet: None,
                        error: None,
                    },
                );
            }
        }
        let this = self.clone();
        let task = tokio::spawn(async move {
            for f in folders.into_iter().filter(|f| f.bittorrent) {
                let result = this.seed(PathBuf::from(&f.path), generation).await;
                if this.generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                if let Some(s) = this.statuses.lock().unwrap().get_mut(&f.path) {
                    match result {
                        Ok(uri) => {
                            s.state = if f.ed2k {
                                "Compartiendo en ambas redes"
                            } else {
                                "Compartiendo en BitTorrent"
                            }
                            .into();
                            s.magnet = Some(uri)
                        }
                        Err(e) => {
                            s.state = "Error en BitTorrent".into();
                            s.error = Some(e)
                        }
                    }
                }
            }
        });
        *self.job.lock().unwrap() = Some(task);
        Ok(())
    }
    async fn seed(&self, path: PathBuf, generation: u64) -> Result<String, String> {
        let root = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
        let torrent = librqbit::create_torrent(
            &root,
            Default::default(),
            &librqbit::spawn_utils::BlockingSpawner::new(1),
        )
        .await
        .map_err(|e| e.to_string())?;
        let uri = torrent.as_magnet().to_string();
        let bytes = torrent.as_bytes().map_err(|e| e.to_string())?;
        let guard = self.control.lock().await;
        if self.generation.load(Ordering::SeqCst) != generation {
            return Err("Compartición cancelada".into());
        }
        let session = self.bt.session().await?;
        let response = session
            .add_torrent(
                librqbit::AddTorrent::from_bytes(bytes),
                Some(librqbit::AddTorrentOptions {
                    paused: true,
                    output_folder: Some(root.to_string_lossy().into_owned()),
                    storage_factory: Some(ReadOnlyFactory { root }.boxed()),
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| e.to_string())?;
        let h = match response {
            librqbit::AddTorrentResponse::Added(_, h) => h,
            _ => {
                return Err(
                    "Este torrent ya está gestionado por una descarga u otra carpeta".into(),
                )
            }
        };
        let key = path.to_string_lossy().into_owned();
        self.handles.lock().await.insert(key.clone(), h.clone());
        drop(guard);
        let initialized = h.wait_until_initialized().await.map_err(|e| e.to_string());
        let _guard = self.control.lock().await;
        if self.generation.load(Ordering::SeqCst) != generation {
            return Err("Compartición cancelada".into());
        }
        if let Err(e) = initialized {
            self.handles.lock().await.remove(&key);
            let _ = session
                .delete(librqbit::api::TorrentIdOrHash::Id(h.id()), false)
                .await;
            return Err(e);
        }
        if h.stats().progress_bytes != h.stats().total_bytes {
            self.handles.lock().await.remove(&key);
            let _ = session
                .delete(librqbit::api::TorrentIdOrHash::Id(h.id()), false)
                .await;
            return Err("Los archivos han cambiado; actualiza la compartición".into());
        }
        session.unpause(&h).await.map_err(|e| e.to_string())?;
        Ok(uri)
    }
    pub async fn shutdown(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(job) = self.job.lock().unwrap().take() {
            job.abort()
        }
        let _guard = self.control.lock().await;
        let handles = std::mem::take(&mut *self.handles.lock().await);
        if let Some(session) = self.bt.current_session() {
            for (_, handle) in handles {
                let _ = session
                    .delete(librqbit::api::TorrentIdOrHash::Id(handle.id()), false)
                    .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn shared_folder_is_read_only_and_can_be_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("shared");
        std::fs::create_dir(&root).unwrap();
        let payload = vec![42u8; 65536];
        std::fs::write(root.join("test.bin"), &payload).unwrap();
        let bt = crate::downloads::Downloads::new(tmp.path().join("bt")).unwrap();
        let ed = crate::ed2k_downloads::Downloads::new(
            PathBuf::from("not-started.exe"),
            tmp.path().join("ed"),
        )
        .unwrap();
        let shares = Sharing::new(bt.clone(), ed);
        let folder = Folder {
            path: root.to_str().unwrap().into(),
            bittorrent: true,
            ed2k: false,
        };
        shares.configure(vec![folder.clone()], false).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(25), async {
            loop {
                let states = shares.list();
                if states[0].magnet.is_some() {
                    break;
                }
                if let Some(e) = &states[0].error {
                    panic!("{e}")
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        })
        .await
        .unwrap();
        assert!(shares.bt_active());
        assert_eq!(std::fs::read(root.join("test.bin")).unwrap(), payload);
        assert!(shares
            .configure(vec![folder.clone(), folder], false)
            .await
            .is_err());
        shares.configure(vec![], false).await.unwrap();
        assert!(!shares.bt_active());
        assert_eq!(std::fs::read(root.join("test.bin")).unwrap(), payload);
        shares.shutdown().await;
        bt.shutdown().await;
    }
}
