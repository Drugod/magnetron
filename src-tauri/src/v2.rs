//! Pure BEP 52 transfers through an isolated, bundled libtorrent worker over stdio.
use crate::downloads::Download;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{BufRead, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    row: Download,
    files: Vec<PathBuf>,
}
struct Running {
    child: Child,
    input: ChildStdin,
    token: u64,
    last_limits: (u32, u32),
    _job: crate::engine::Job,
}
pub struct Downloads {
    runtime: PathBuf,
    data: PathBuf,
    records: Mutex<BTreeMap<String, Entry>>,
    jobs: Mutex<BTreeMap<String, Running>>,
    control: Mutex<()>,
    limits: Mutex<(u32, u32)>,
}
pub fn identity(uri: &str) -> Result<(String, String), String> {
    let safe = crate::model::validate_magnet(uri)?;
    let parsed = url::Url::parse(&safe).map_err(|e| e.to_string())?;
    if parsed
        .query_pairs()
        .any(|(k, v)| k == "xt" && v.starts_with("urn:btih:"))
    {
        return Err("Este enlace también tiene identidad v1".into());
    }
    let hash = parsed
        .query_pairs()
        .find_map(|(k, v)| {
            (k == "xt")
                .then(|| v.strip_prefix("urn:btmh:1220").map(str::to_lowercase))
                .flatten()
        })
        .ok_or("Falta el hash BitTorrent v2")?;
    if parsed
        .query_pairs()
        .filter(|(k, _)| k == "xt")
        .any(|(_, v)| {
            v.strip_prefix("urn:btmh:1220")
                .is_none_or(|h| h.to_lowercase() != hash)
        })
    {
        return Err("Identidades v2 contradictorias".into());
    }
    let mut clean = url::Url::parse("magnet:").unwrap();
    clean
        .query_pairs_mut()
        .append_pair("xt", &format!("urn:btmh:1220{hash}"));
    for (k, v) in parsed.query_pairs() {
        if k == "dn"
            || (k == "x.pe" && v.parse::<std::net::SocketAddr>().is_ok())
            || (k == "tr"
                && url::Url::parse(&v).is_ok_and(|u| {
                    matches!(u.scheme(), "https" | "http" | "udp")
                        && u.host_str().is_some()
                        && u.username().is_empty()
                        && u.password().is_none()
                }))
        {
            clean.query_pairs_mut().append_pair(&k, &v);
        }
    }
    Ok((format!("bt2:{hash}"), clean.to_string()))
}
pub fn pure(uri: &str) -> bool {
    identity(uri).is_ok()
}
fn stop(mut job: Running) {
    let _ = writeln!(job.input, "{{\"op\":\"stop\"}}");
    let _ = job.input.flush();
    for _ in 0..40 {
        if job.child.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = job.child.kill();
    let _ = job.child.wait();
}
impl Downloads {
    pub fn new(runtime: PathBuf, data: PathBuf) -> Result<Arc<Self>, String> {
        std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
        let file = data.join("downloads.json");
        let mut records: BTreeMap<String, Entry> = if file.exists() {
            serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        } else {
            BTreeMap::new()
        };
        for e in records.values_mut() {
            if !matches!(e.row.state.as_str(), "completed" | "cancelled") {
                e.row.state = "paused".into()
            }
            e.row.download_speed = 0;
            e.row.upload_speed = 0;
        }
        Ok(Arc::new(Self {
            runtime,
            data,
            records: Mutex::new(records),
            jobs: Mutex::new(BTreeMap::new()),
            control: Mutex::new(()),
            limits: Mutex::new((0, 0)),
        }))
    }
    fn save(&self) -> Result<(), String> {
        let bytes =
            serde_json::to_vec(&*self.records.lock().unwrap()).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(&self.data).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.persist(self.data.join("downloads.json"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn list(&self) -> Vec<Download> {
        self.records
            .lock()
            .unwrap()
            .values()
            .map(|e| {
                let mut r = e.row.clone();
                r.uri.clear();
                r
            })
            .collect()
    }
    pub fn active(&self) -> bool {
        !self.jobs.lock().unwrap().is_empty()
    }
    pub fn folder(&self, id: &str) -> Result<PathBuf, String> {
        self.records
            .lock()
            .unwrap()
            .get(id)
            .map(|e| PathBuf::from(&e.row.folder))
            .ok_or("Descarga no encontrada".into())
    }
    pub fn set_limits(&self, down: u32, up: u32) {
        let mut limits = self.limits.lock().unwrap();
        let mut jobs = self.jobs.lock().unwrap();
        let n = jobs.len().max(1) as u32;
        let portion = |v: u32| if v == 0 { 0 } else { (v / n).max(1) };
        if *limits == (down, up) && jobs.is_empty() {
            return;
        }
        *limits = (down, up);
        for job in jobs.values_mut() {
            let desired = (portion(down), portion(up));
            if job.last_limits == desired {
                continue;
            }
            job.last_limits = desired;
            let _ = writeln!(
                job.input,
                "{}",
                serde_json::json!({"op":"limits","down":portion(down),"up":portion(up)})
            );
            let _ = job.input.flush();
        }
    }
    pub fn add(
        self: &Arc<Self>,
        uri: String,
        name: String,
        folder: PathBuf,
    ) -> Result<String, String> {
        let (id, uri) = identity(&uri)?;
        let _guard = self.control.lock().unwrap();
        if self.records.lock().unwrap().contains_key(&id) {
            return Ok(id);
        }
        let folder = folder.canonicalize().map_err(|e| e.to_string())?;
        crate::downloads::reject_links(&folder)?;
        self.records.lock().unwrap().insert(
            id.clone(),
            Entry {
                row: Download {
                    id: id.clone(),
                    uri,
                    name,
                    folder: folder.to_str().ok_or("Ruta inválida")?.into(),
                    state: "resolving".into(),
                    completed_bytes: 0,
                    total_bytes: 0,
                    download_speed: 0,
                    upload_speed: 0,
                    error: None,
                },
                files: vec![],
            },
        );
        self.save()?;
        if let Err(error) = self.spawn(&id, false) {
            self.records.lock().unwrap().get_mut(&id).unwrap().row.state = "error".into();
            self.records.lock().unwrap().get_mut(&id).unwrap().row.error = Some(error);
            self.save()?;
        }
        Ok(id)
    }
    fn spawn(self: &Arc<Self>, id: &str, resume: bool) -> Result<(), String> {
        if self.jobs.lock().unwrap().len() >= 8 {
            return Err("Máximo 8 descargas v2 simultáneas; reintenta cuando termine otra".into());
        }
        let e = self
            .records
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        let staging = self
            .data
            .join("staging")
            .join(id.trim_start_matches("bt2:"));
        std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
        let mut cmd = Command::new(crate::platform::python(&self.runtime));
        cmd.arg("-I")
            .arg("-u")
            .arg(self.runtime.join("worker.py"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("No se pudo iniciar BitTorrent v2: {e}"))?;
        let job = match crate::engine::Job::attach(&child) {
            Ok(job) => job,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        };
        let mut input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (down, up) = *self.limits.lock().unwrap();
        writeln!(input,"{}",serde_json::json!({"uri":e.row.uri,"folder":e.row.folder,"staging":staging,"resume":resume,"down":down,"up":up})).map_err(|e|e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        let token = rand::random::<u64>();
        self.jobs.lock().unwrap().insert(
            id.into(),
            Running {
                child,
                input,
                token,
                last_limits: (down, up),
                _job: job,
            },
        );
        let weak = Arc::downgrade(self);
        let id = id.to_owned();
        std::thread::spawn(move || {
            let mut completed = false;
            let mut error = None;
            for line in std::io::BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if line.len() > 16 * 1024 * 1024 {
                    error = Some("Respuesta v2 demasiado grande".into());
                    break;
                }
                let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                let Some(this) = weak.upgrade() else { return };
                let _guard = this.control.lock().unwrap();
                if this
                    .jobs
                    .lock()
                    .unwrap()
                    .get(&id)
                    .is_none_or(|j| j.token != token)
                {
                    return;
                }
                let mut records = this.records.lock().unwrap();
                let Some(e) = records.get_mut(&id) else {
                    return;
                };
                match message["kind"].as_str() {
                    Some("metadata") => {
                        let files: Vec<PathBuf> =
                            serde_json::from_value(message["files"].clone()).unwrap_or_default();
                        if files
                            .iter()
                            .any(|p| crate::downloads::validate_file_path(p).is_err())
                        {
                            error = Some("Metadatos v2 con rutas inválidas".into());
                            break;
                        }
                        e.files = files;
                        e.row.total_bytes = message["total"].as_u64().unwrap_or(0);
                    }
                    Some("progress") => {
                        e.row.completed_bytes = message["completed"].as_u64().unwrap_or(0);
                        e.row.total_bytes = message["total"].as_u64().unwrap_or(e.row.total_bytes);
                        e.row.download_speed = message["down"].as_u64().unwrap_or(0);
                        e.row.upload_speed = message["up"].as_u64().unwrap_or(0);
                        e.row.state = message["state"].as_str().unwrap_or("downloading").into();
                    }
                    Some("completed") => completed = true,
                    Some("error") => {
                        error = Some(
                            message["error"]
                                .as_str()
                                .unwrap_or("Error BitTorrent v2")
                                .to_owned(),
                        )
                    }
                    _ => {}
                }
                drop(records);
                let _ = this.save();
            }
            let Some(this) = weak.upgrade() else { return };
            let _guard = this.control.lock().unwrap();
            if this
                .jobs
                .lock()
                .unwrap()
                .get(&id)
                .is_none_or(|j| j.token != token)
            {
                return;
            }
            if let Some(mut job) = this.jobs.lock().unwrap().remove(&id) {
                if error.is_some() {
                    let _ = job.child.kill();
                }
                let success = job.child.wait().is_ok_and(|s| s.success());
                completed &= success;
            }
            if let Some(e) = this.records.lock().unwrap().get_mut(&id) {
                e.row.state = if completed { "completed" } else { "error" }.into();
                e.row.download_speed = 0;
                e.row.upload_speed = 0;
                e.row.error = if completed {
                    None
                } else {
                    Some(error.unwrap_or("El motor v2 se ha detenido".into()))
                };
                if completed {
                    e.row.completed_bytes = e.row.total_bytes;
                }
            }
            let _ = this.save();
        });
        Ok(())
    }
    pub fn action(self: &Arc<Self>, id: &str, action: &str) -> Result<(), String> {
        let _guard = self.control.lock().unwrap();
        if !matches!(action, "pause" | "cancel" | "resume") {
            return Err("Acción desconocida".into());
        }
        let old = self
            .records
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        if old.row.state == "completed" {
            return Err("La descarga ya ha terminado".into());
        }
        if let Some(job) = self.jobs.lock().unwrap().remove(id) {
            stop(job)
        }
        if action == "resume" {
            self.spawn(id, true)?;
        }
        let mut records = self.records.lock().unwrap();
        let row = &mut records.get_mut(id).unwrap().row;
        row.state = match action {
            "resume" => "resolving",
            "pause" => "paused",
            _ => "cancelled",
        }
        .into();
        row.error = None;
        row.download_speed = 0;
        row.upload_speed = 0;
        drop(records);
        self.save()
    }
    pub fn remove(&self, id: &str, delete_files: bool) -> Result<(), String> {
        let _guard = self.control.lock().unwrap();
        if let Some(job) = self.jobs.lock().unwrap().remove(id) {
            stop(job)
        }
        let e = self
            .records
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Descarga no encontrada")?;
        if delete_files {
            crate::downloads::delete_payload(std::path::Path::new(&e.row.folder), &e.files)?;
        }
        self.records.lock().unwrap().remove(id);
        self.save()
    }
    pub fn shutdown(&self) {
        let _guard = self.control.lock().unwrap();
        for (_, job) in std::mem::take(&mut *self.jobs.lock().unwrap()) {
            stop(job)
        }
        for e in self.records.lock().unwrap().values_mut() {
            if !matches!(e.row.state.as_str(), "completed" | "cancelled") {
                e.row.state = "paused".into()
            }
            e.row.download_speed = 0;
            e.row.upload_speed = 0;
        }
        let _ = self.save();
    }
}

/// Worker timeout is enforced internally; Windows Job releases it if the app exits.
pub fn verify(
    runtime: &std::path::Path,
    mut item: crate::model::Torrent,
) -> Result<crate::model::Torrent, String> {
    let (_, safe) = identity(&item.magnet_uri)?;
    // Search evidence uses public DHT, without following indexer-supplied tracker/peer endpoints.
    let parsed = url::Url::parse(&safe).map_err(|e| e.to_string())?;
    let mut uri = url::Url::parse("magnet:").unwrap();
    for (k, v) in parsed.query_pairs().filter(|(k, _)| k == "xt" || k == "dn") {
        uri.query_pairs_mut().append_pair(&k, &v);
    }
    let uri = uri.to_string();
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(crate::platform::python(runtime));
    cmd.arg("-I")
        .arg("-u")
        .arg(runtime.join("worker.py"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let _job = match crate::engine::Job::attach(&child) {
        Ok(job) => job,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    let request = serde_json::json!({"uri":uri,"folder":temp.path(),"staging":temp.path(),"verify":true,"down":32,"up":16});
    let mut input = child.stdin.take().unwrap();
    if let Err(e) = writeln!(input, "{request}") {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e.to_string());
    }
    // Keep stdin open until completion; the worker treats EOF as cancellation.

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let mut verified = false;
    for line in output.stdout.split(|b| *b == b'\n') {
        let Ok(event) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        match event["kind"].as_str() {
            Some("metadata") => {
                item.title = event["name"].as_str().unwrap_or(&item.title).to_owned();
                item.size_bytes = event["total"].as_u64();
                item.files = serde_json::from_value(event["files"].clone()).unwrap_or_default();
            }
            Some("verified") => verified = true,
            _ => {}
        }
    }
    if !verified || !output.status.success() {
        return Err("No se ha encontrado una fuente v2 completa accesible".into());
    }
    item.availability_status = "verified_v2".into();
    item.availability_checked_at = Some(chrono::Utc::now().to_rfc3339());
    Ok(item)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_v2_and_hybrid() {
        let uri = format!(
            "magnet:?xt=urn:btmh:1220{}&dn=sample&ws=file:///x",
            "ab".repeat(32)
        );
        let (id, safe) = identity(&uri).unwrap();
        assert_eq!(id, format!("bt2:{}", "ab".repeat(32)));
        assert!(!safe.contains("file:"));
        assert!(identity(&format!("{uri}&xt=urn:btih:{}", "a".repeat(40))).is_err());
    }
    #[test]
    fn native_v2_pause_resume_and_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let runtime = crate::platform::dev_runtime();
        let seed_dir = tmp.path().join("seed");
        std::fs::create_dir(&seed_dir).unwrap();
        let mut child = Command::new(crate::platform::python(&runtime))
            .arg("-I")
            .arg("-u")
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/test-v2.py"))
            .arg("--seed")
            .arg(&seed_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let _job = crate::engine::Job::attach(&child).unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut uri = String::new();
        output.read_line(&mut uri).unwrap();
        assert!(pure(uri.trim()));
        let dest = tmp.path().join("downloads");
        std::fs::create_dir(&dest).unwrap();
        let data = tmp.path().join("data");
        let manager = Downloads::new(runtime.clone(), data.clone()).unwrap();
        manager.set_limits(32, 32);
        let id = manager
            .add(uri.trim().into(), "sample.bin".into(), dest.clone())
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        while manager.list()[0].total_bytes == 0 {
            assert!(std::time::Instant::now() < deadline);
            assert!(
                manager.list()[0].error.is_none(),
                "{:?}",
                manager.list()[0].error
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        manager.action(&id, "pause").unwrap();
        assert_eq!(manager.list()[0].state, "paused");
        manager.shutdown();
        drop(manager);
        let manager = Downloads::new(runtime, data).unwrap();
        assert_eq!(manager.list()[0].state, "paused");
        manager.set_limits(0, 0);
        manager.action(&id, "resume").unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let row = manager.list().remove(0);
            if row.state == "completed" {
                break;
            }
            assert!(row.error.is_none(), "{:?}", row.error);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(100));
        }
        assert_eq!(
            std::fs::read(dest.join("sample.bin")).unwrap(),
            std::fs::read(seed_dir.join("sample.bin")).unwrap()
        );
        std::fs::write(dest.join("keep.txt"), b"keep").unwrap();
        manager.remove(&id, true).unwrap();
        assert!(!dest.join("sample.bin").exists());
        assert!(dest.join("keep.txt").exists());
        manager.shutdown();
        let _ = child.kill();
        let _ = child.wait();
    }
}
