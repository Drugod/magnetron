#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app_updates;
mod availability;
mod dht;
mod downloads;
mod ec;
mod ed2k;
mod ed2k_downloads;
mod engine;
mod engine_updates;
mod exports;
mod kad;
mod links;
mod model;
mod platform;
mod sharing;
mod sources;
mod storage;
mod torrent;
mod tray;
mod v2;
use futures_util::{stream, StreamExt};
use model::Torrent;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use storage::{Settings, Storage};
use tauri::{Emitter, Manager, State};

struct AppState {
    storage: Arc<Mutex<Storage>>,
    running: Mutex<Option<(String, tokio::sync::watch::Sender<bool>)>>,
    engine: Arc<engine::Engine>,
    discovery: dht::Discovery,
    verifier: Arc<availability::Verifier>,
    downloads: Arc<downloads::Downloads>,
    ed2k_downloads: Arc<ed2k_downloads::Downloads>,
    sharing: Arc<sharing::Sharing>,
    v2_downloads: Arc<v2::Downloads>,
}
#[derive(Clone, Serialize)]
struct SearchEvent {
    search_id: String,
    kind: String,
    results: Vec<Torrent>,
    source: Option<String>,
    message: Option<String>,
}
fn event(
    id: &str,
    kind: &str,
    results: Vec<Torrent>,
    source: Option<String>,
    message: Option<String>,
) -> SearchEvent {
    SearchEvent {
        search_id: id.into(),
        kind: kind.into(),
        results,
        source,
        message,
    }
}
#[tauri::command]
fn list_favorites(state: State<AppState>) -> Result<Vec<Torrent>, String> {
    state.storage.lock().map_err(|e| e.to_string())?.favorites()
}
#[tauri::command]
fn set_favorite(torrent: Torrent, enabled: bool, state: State<AppState>) -> Result<(), String> {
    state
        .storage
        .lock()
        .map_err(|e| e.to_string())?
        .favorite(&torrent, enabled)
}
#[tauri::command]
fn sharing_status(state: State<AppState>) -> Vec<sharing::Status> {
    state.sharing.list()
}
#[tauri::command]
async fn refresh_sharing(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state
        .storage
        .lock()
        .map_err(|e| e.to_string())?
        .settings()?;
    state.sharing.configure(cfg.shared_folders, true).await
}
#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    state.storage.lock().map_err(|e| e.to_string())?.settings()
}
#[tauri::command]
fn set_download_folder(folder: String, state: State<AppState>) -> Result<Settings, String> {
    let path = std::path::Path::new(&folder);
    if !folder.is_empty() && (!path.is_absolute() || !path.is_dir()) {
        return Err("Elige una carpeta existente".into());
    }
    let db = state.storage.lock().map_err(|e| e.to_string())?;
    let mut cfg = db.settings()?;
    cfg.download_folder = folder;
    db.save_settings(&cfg)?;
    Ok(cfg)
}
async fn update_limits(state: &AppState) -> Result<(), String> {
    let cfg = state
        .storage
        .lock()
        .map_err(|e| e.to_string())?
        .settings()?;
    let active = |rows: Vec<downloads::Download>| {
        rows.iter().any(|r| {
            matches!(
                r.state.as_str(),
                "resolving" | "checking" | "downloading" | "waiting" | "finalizing"
            )
        })
    };
    let bt = active(state.downloads.list()) || state.sharing.bt_active();
    let ed = active(state.ed2k_downloads.list()) || state.sharing.ed_active();
    let v2 = state.v2_downloads.active();
    let groups = (bt as u32 + ed as u32 + v2 as u32).max(1);
    let portion = |n: u32| if n == 0 { 0 } else { (n / groups).max(1) };
    let (down, up) = (
        portion(cfg.download_limit_kib),
        portion(cfg.upload_limit_kib),
    );
    let bt_down = if bt && down > 0 { down.max(16) } else { down };
    let bt_up = if bt && up > 0 { up.max(16) } else { up };
    state.downloads.set_limits(bt_down, bt_up);
    let other = |n: u32, bt_share: u32| {
        if n == 0 {
            0
        } else if bt && groups > 1 {
            ((n.saturating_sub(bt_share)) / (groups - 1)).max(1)
        } else {
            portion(n)
        }
    };
    let (down, up) = (
        other(cfg.download_limit_kib, bt_down),
        other(cfg.upload_limit_kib, bt_up),
    );
    state.v2_downloads.set_limits(down, up);
    state.ed2k_downloads.set_limits(down, up).await
}
#[tauri::command]
async fn choose_download_folder(
    language: Option<String>,
    purpose: Option<String>,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title(
                match (
                    language.as_deref() == Some("en"),
                    purpose.as_deref() == Some("share"),
                ) {
                    (true, true) => "Choose a folder to share",
                    (false, true) => "Elige una carpeta para compartir",
                    (true, false) => "Where do you want to save your downloads?",
                    _ => "¿Dónde quieres guardar la descarga?",
                },
            )
            .pick_folder()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())
}
#[tauri::command]
fn set_window_appearance(theme: String, window: tauri::WebviewWindow) -> Result<(), String> {
    window
        .set_theme(match theme.as_str() {
            "dark" => Some(tauri::Theme::Dark),
            "light" => Some(tauri::Theme::Light),
            _ => None,
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn list_downloads(state: State<AppState>) -> Vec<downloads::Download> {
    let mut rows = state.downloads.list();
    rows.extend(state.ed2k_downloads.list());
    rows.extend(state.v2_downloads.list());
    rows
}
#[tauri::command]
async fn add_download(
    uri: String,
    name: String,
    folder: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let folder = folder.filter(|f| !f.is_empty()).unwrap_or(
        state
            .storage
            .lock()
            .map_err(|e| e.to_string())?
            .settings()?
            .download_folder,
    );
    if folder.is_empty() {
        return Err("Elige primero la carpeta de descarga en Ajustes".into());
    }
    update_limits(&state).await?;
    if v2::pure(&uri) {
        return state
            .v2_downloads
            .add(uri, name, std::path::PathBuf::from(folder));
    }
    if uri.starts_with("ed2k://") {
        return state
            .ed2k_downloads
            .add(uri, name, std::path::PathBuf::from(folder))
            .await;
    }
    state
        .downloads
        .add(uri, name, std::path::PathBuf::from(folder))
        .await
}
#[tauri::command]
async fn remove_download(
    id: String,
    delete_files: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if id.starts_with("bt2:") {
        return state.v2_downloads.remove(&id, delete_files);
    }
    if id.starts_with("ed2k:") {
        state.ed2k_downloads.remove(&id, delete_files).await
    } else {
        state.downloads.remove(&id, delete_files).await
    }
}
#[tauri::command]
async fn download_action(
    id: String,
    action: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if id.starts_with("bt2:") {
        return state.v2_downloads.action(&id, &action);
    }
    if id.starts_with("ed2k:") {
        return state.ed2k_downloads.action(&id, &action).await;
    }
    state.downloads.action(&id, &action).await
}
#[tauri::command]
async fn open_download_folder(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let folder = if id.starts_with("bt2:") {
        state.v2_downloads.folder(&id)?
    } else if id.starts_with("ed2k:") {
        state.ed2k_downloads.folder(&id)?
    } else {
        state.downloads.folder(&id)?
    };
    tauri::async_runtime::spawn_blocking(move || {
        open::that_detached(folder).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    settings.validate()?;
    state
        .sharing
        .configure(settings.shared_folders.clone(), false)
        .await?;
    state
        .storage
        .lock()
        .map_err(|e| e.to_string())?
        .save_settings(&settings)?;
    update_limits(&state).await
}
#[tauri::command]
fn get_history(state: State<AppState>) -> Result<Vec<String>, String> {
    state.storage.lock().map_err(|e| e.to_string())?.history()
}
#[tauri::command]
fn clear_history(state: State<AppState>) -> Result<(), String> {
    state
        .storage
        .lock()
        .map_err(|e| e.to_string())?
        .clear_history()
}
#[tauri::command]
fn cancel_search(state: State<AppState>) -> Result<(), String> {
    if let Some((_, tx)) = state.running.lock().map_err(|e| e.to_string())?.take() {
        let _ = tx.send(true);
    }
    Ok(())
}
#[tauri::command]
async fn clear_data(state: State<'_, AppState>) -> Result<(), String> {
    {
        let db = state.storage.lock().map_err(|e| e.to_string())?;
        let mut cfg = db.settings()?;
        cfg.discovery_enabled = false;
        db.save_settings(&cfg)?;
    }
    state.discovery.pause().await;
    cancel_search(state.clone())?;
    state.storage.lock().map_err(|e| e.to_string())?.clear()
}
#[tauri::command]
async fn list_indexers(state: State<'_, AppState>) -> Result<Vec<engine::Indexer>, String> {
    state.engine.catalog().await
}
#[tauri::command]
async fn start_dht(state: State<'_, AppState>) -> Result<(), String> {
    {
        let db = state.storage.lock().map_err(|e| e.to_string())?;
        let mut cfg = db.settings()?;
        cfg.discovery_enabled = true;
        db.save_settings(&cfg)?;
    }
    state.discovery.start(state.storage.clone()).await
}
#[tauri::command]
async fn pause_dht(state: State<'_, AppState>) -> Result<(), String> {
    {
        let db = state.storage.lock().map_err(|e| e.to_string())?;
        let mut cfg = db.settings()?;
        cfg.discovery_enabled = false;
        db.save_settings(&cfg)?;
    }
    state.discovery.pause().await;
    Ok(())
}
#[derive(Serialize)]
struct IndexStatus {
    dht: dht::Status,
    records: u64,
}
#[tauri::command]
fn index_status(state: State<AppState>) -> Result<IndexStatus, String> {
    Ok(IndexStatus {
        dht: state.discovery.status(),
        records: state.storage.lock().map_err(|e| e.to_string())?.count()?,
    })
}
#[tauri::command]
async fn open_magnet(uri: String, state: State<'_, AppState>) -> Result<(), String> {
    let safe = model::validate_magnet(&uri)?;
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
        .ok_or("Falta hash v1")?;
    let hash = if hash.len() == 32 {
        hex::encode(
            data_encoding::BASE32
                .decode(hash.to_uppercase().as_bytes())
                .map_err(|_| "Hash inválido")?,
        )
    } else {
        hash
    };
    let candidate = Torrent {
        id: hash.clone(),
        infohash_v1: hash,
        title: String::new(),
        magnet_uri: safe.clone(),
        size_bytes: None,
        published_at: None,
        seeds_reported: None,
        availability_status: "unknown".into(),
        availability_checked_at: None,
        sources: vec![],
        files: vec![],
        content_types: vec![],
        network: "bittorrent".into(),
        ed2k_uri: None,
        complete_sources_reported: None,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    };
    state
        .verifier
        .check(candidate)
        .await
        .map_err(|e| format!("La disponibilidad ha cambiado. No se abre la descarga: {e}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        open::that_detached(safe)
            .map_err(|e| format!("No se pudo abrir el cliente BitTorrent: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn open_ed2k(uri: String, state: State<'_, AppState>) -> Result<(), String> {
    let safe = ed2k::validate_link(&uri)?;
    let fields: Vec<_> = safe.split('|').collect();
    let id = format!("ed2k:{}:{}", fields[4], fields[3]);
    let item = state
        .storage
        .lock()
        .map_err(|e| e.to_string())?
        .find(&id)?
        .ok_or("Repite la búsqueda para comprobar las fuentes eD2k")?;
    ed2k::recheck(&item).await?;
    tauri::async_runtime::spawn_blocking(move || {
        open::that_detached(safe).map_err(|e| format!("No se pudo abrir eMule/aMule: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn start_search(
    query: String,
    search_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let query = query.trim().to_string();
    if query.is_empty() || query.len() > 200 || search_id.len() > 100 {
        return Err("La consulta debe tener entre 1 y 200 caracteres".into());
    }
    cancel_search(state.clone())?;
    let settings = state
        .storage
        .lock()
        .map_err(|e| e.to_string())?
        .settings()?;
    settings.validate()?;
    let cached = {
        let db = state.storage.lock().map_err(|e| e.to_string())?;
        if settings.history {
            db.remember(&query)?;
        }
        db.search(&query, &settings)?
    };
    let (tx, mut cancel) = tokio::sync::watch::channel(false);
    *state.running.lock().map_err(|e| e.to_string())? = Some((search_id.clone(), tx));
    let db = state.storage.clone();
    let engine = state.engine.clone();
    let kad_engine = state.ed2k_downloads.clone();
    let verifier = state.verifier.clone();
    tauri::async_runtime::spawn(async move {
        let (candidates, receiver) = tokio::sync::mpsc::unbounded_channel::<Torrent>();
        let seen = Arc::new(Mutex::new(std::collections::HashSet::new()));
        for item in cached {
            if item.network == "ed2k" {
                continue;
            }
            if seen.lock().unwrap().insert(item.id.clone()) {
                let _ = candidates.send(item);
            }
        }
        let client = match sources::client() {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit(
                    "search-event",
                    event(&search_id, "error", vec![], None, Some(e)),
                );
                return;
            }
        };
        let mut enabled: Vec<_> = sources::registry()
            .into_iter()
            .filter(|s| match s.id() {
                "ubuntu" => settings.ubuntu,
                "debian" => settings.debian,
                "archive" => settings.archive,
                _ => false,
            })
            .collect();
        if settings.ed2k {
            enabled.insert(0, Box::new(ed2k::Ed2k));
        }
        if settings.kad {
            enabled.insert(0, Box::new(kad::KadSource { engine: kad_engine }));
        }
        let indexer_ids = if settings.auto_sources {
            engine
                .automatic_sources()
                .await
                .unwrap_or_else(|_| settings.indexers.clone())
        } else {
            settings.indexers.clone()
        };
        for id in &indexer_ids {
            enabled.push(Box::new(engine::EngineSource {
                engine: engine.clone(),
                id: format!("indexer:{id}"),
            }));
        }

        let mut pending = vec![];
        for source in enabled {
            let app = app.clone();
            let search_id = search_id.clone();
            let query = query.clone();
            let client = client.clone();
            let db = db.clone();
            let candidates = candidates.clone();
            let seen = seen.clone();
            let mut cancelled = cancel.clone();
            pending.push(async move {
                let id=source.id().to_string();
                let _=app.emit("search-event",event(&search_id,"source-start",vec![],Some(id.clone()),None));
                let a=app.clone();let sid=search_id.clone();let active=cancelled.clone();
                let batch:sources::Batch=Arc::new(move|items|{
                    if *active.borrow(){return}

                    if let Ok(db)=db.lock(){if *active.borrow(){return}if let Err(e)=db.cache(&items){let _=a.emit("search-event",event(&sid,"warning",vec![],None,Some(format!("No se pudo guardar la caché: {e}"))));}}
                    for item in items {
                        if item.network == "ed2k" {
                            if item.complete_sources_reported.unwrap_or(0)>0 {
                                let _=a.emit("search-event",event(&sid,"batch",vec![item],None,None));
                            }
                            continue;
                        }
                        if seen.lock().unwrap().insert(item.id.clone()) { let _ = candidates.send(item); }
                    }
                });
                let result=tokio::select!{
                    _=cancelled.changed()=>return,
                    result=tokio::time::timeout(std::time::Duration::from_secs(if id=="kad"{120}else{settings.timeout_seconds}),source.search(&client,&query,batch))=>result
                };
                let (kind,message)=match result{Ok(Ok(n))=>("source-done",format!("{n} resultados")),Ok(Err(e))=>("source-error",e),Err(_)=>("source-error","Tiempo máximo agotado; se conservan los resultados recibidos".into())};
                let _=app.emit("search-event",event(&search_id,kind,vec![],Some(id),Some(message)));
            });
        }
        drop(candidates);
        let collect = async {
            let jobs = stream::iter(pending).buffer_unordered(settings.concurrency);
            tokio::pin!(jobs);
            while jobs.next().await.is_some() {}
        };
        let verification_active = cancel.clone();
        let verify = async {
            let input = stream::unfold(receiver, |mut rx| async move {
                rx.recv().await.map(|item| (item, rx))
            });
            let jobs = input
                .map(|item| {
                    let verifier = verifier.clone();
                    async move { verifier.check(item).await }
                })
                .buffer_unordered(4);
            tokio::pin!(jobs);
            let mut examined = 0;
            let mut admitted = 0;
            while let Some(result) = jobs.next().await {
                if *verification_active.borrow() {
                    return;
                }
                examined += 1;
                if let Ok(item) = result {
                    admitted += 1;
                    if let Ok(db) = db.lock() {
                        if *verification_active.borrow() {
                            return;
                        }
                        let _ = db.cache(std::slice::from_ref(&item));
                    }
                    let _ = app.emit(
                        "search-event",
                        event(&search_id, "batch", vec![item], None, None),
                    );
                }
                let _ = app.emit(
                    "search-event",
                    event(
                        &search_id,
                        "checking",
                        vec![],
                        None,
                        Some(format!(
                            "Comprobando BitTorrent · {examined} revisados · {admitted} accesibles"
                        )),
                    ),
                );
            }
        };
        let work = async {
            tokio::join!(collect, verify);
        };
        let outcome = tokio::select! {
            _ = cancel.changed() => "cancelled",
            result = tokio::time::timeout(std::time::Duration::from_secs(180), work) => if result.is_ok() { "done" } else { "limited" },
        };
        let _ = app.emit(
            "search-event",
            event(&search_id, outcome, vec![], None, None),
        );
    });
    Ok(())
}
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            links::enqueue(app, args);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            links::enqueue(app.handle(), std::env::args().skip(1));
            use tauri_plugin_deep_link::DeepLinkExt;
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                links::enqueue(app.handle(), urls.into_iter().map(|u| u.to_string()));
            }
            let link_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                links::enqueue(
                    &link_handle,
                    event.urls().into_iter().map(|u| u.to_string()),
                );
                tray::restore(&link_handle);
            });
            #[cfg(not(debug_assertions))]
            {
                let _ = links::register_link_handlers(app.handle().clone());
            }
            let dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let storage =
                Storage::new(&dir.join("magnet.sqlite")).map_err(std::io::Error::other)?;
            let mut executable = app
                .path()
                .resource_dir()?
                .join("engine")
                .join(platform::jackett_name());
            #[cfg(debug_assertions)]
            if !executable.is_file() {
                executable = platform::dev_jackett();
            }
            let engine = engine::Engine::new(executable, dir.join("engine"));
            let update_data = dir.join("engine");
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                loop {
                    if let Err(error) = engine_updates::check(update_data.clone()).await {
                        let _ = std::fs::write(update_data.join("update-error.txt"), error);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(86400)).await;
                }
            });
            let downloads =
                downloads::Downloads::new(dir.join("downloads")).map_err(std::io::Error::other)?;
            let mut ed2k_executable = app.path().resource_dir()?.join(platform::amule_relative());
            #[cfg(debug_assertions)]
            if !ed2k_executable.is_file() {
                ed2k_executable = platform::dev_amule();
            }
            let ed2k_downloads =
                ed2k_downloads::Downloads::new(ed2k_executable, dir.join("ed2k-downloads"))
                    .map_err(std::io::Error::other)?;
            let sharing = sharing::Sharing::new(downloads.clone(), ed2k_downloads.clone());
            let runtime = if cfg!(debug_assertions) {
                platform::dev_runtime()
            } else {
                app.path().resource_dir()?.join("v2")
            };
            let v2_downloads = v2::Downloads::new(runtime.clone(), dir.join("v2-downloads"))
                .map_err(std::io::Error::other)?;
            let initial_shares = storage
                .settings()
                .map_err(std::io::Error::other)?
                .shared_folders;
            let sharing_task = sharing.clone();
            tauri::async_runtime::spawn(async move {
                let _ = sharing_task.configure(initial_shares, false).await;
            });
            app.manage(AppState {
                storage: Arc::new(Mutex::new(storage)),
                running: Mutex::new(None),
                engine,
                discovery: dht::Discovery::new(),
                verifier: Arc::new(availability::Verifier::new(runtime)),
                downloads,
                ed2k_downloads,
                sharing,
                v2_downloads,
            });
            tray::setup(app.handle())?;
            if app_updates::configured() {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                    loop {
                        let _ = app_updates::check_app_update(handle.clone()).await;
                        tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
                    }
                });
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut next_discovery = std::time::Instant::now();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let _ = update_limits(&handle.state::<AppState>()).await;
                    {
                        use chrono::Timelike;
                        let state = handle.state::<AppState>();
                        let cfg = state.storage.lock().ok().and_then(|db| db.settings().ok());
                        if let Some(cfg) = cfg {
                            let hour = chrono::Local::now().hour();
                            let in_hours = if cfg.discovery_start_hour < cfg.discovery_end_hour {
                                hour >= cfg.discovery_start_hour && hour < cfg.discovery_end_hour
                            } else {
                                hour >= cfg.discovery_start_hour || hour < cfg.discovery_end_hour
                            };
                            let want = cfg.discovery_enabled && in_hours;
                            if want
                                && !state.discovery.status().running
                                && std::time::Instant::now() >= next_discovery
                            {
                                next_discovery =
                                    std::time::Instant::now() + std::time::Duration::from_secs(60);
                                let _ = state.discovery.start(state.storage.clone()).await;
                            } else if !want && state.discovery.status().running {
                                state.discovery.pause().await;
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            links::parse_download_links,
            links::take_download_links,
            links::register_link_handlers,
            set_download_folder,
            choose_download_folder,
            set_window_appearance,
            tray::set_ui_language,
            list_downloads,
            add_download,
            download_action,
            remove_download,
            open_download_folder,
            get_settings,
            list_favorites,
            set_favorite,
            app_updates::check_app_update,
            app_updates::install_app_update,
            sharing_status,
            refresh_sharing,
            exports::export_results,
            save_settings,
            get_history,
            clear_history,
            start_search,
            cancel_search,
            clear_data,
            open_magnet,
            open_ed2k,
            list_indexers,
            start_dht,
            pause_dht,
            index_status
        ])
        .build(tauri::generate_context!())
        .expect("No se pudo iniciar MAGNETRON");
    app.run(|handle, event| {
        #[cfg(target_os = "macos")]
        match event {
            tauri::RunEvent::Reopen { .. } => tray::restore(handle),
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !tray::is_exiting(handle) {
                    api.prevent_exit();
                    tray::exit(handle.clone());
                }
            }
            _ => {}
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (handle, event);
        }
    });
}
