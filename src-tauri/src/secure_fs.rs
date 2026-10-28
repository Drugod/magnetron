//! Private, atomic persistence for application state, never download destinations.
use std::{io::Write, path::Path};
pub fn private_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    if std::fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Application state directory cannot be a symbolic link".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn atomic_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing application state directory")?;
    private_dir(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    // Windows can briefly deny replacement while a reader or antivirus holds
    // the destination. Retry the same atomic rename; never delete the old file.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        match file.persist(path) {
            Ok(_) => break,
            Err(error) => {
                #[cfg(windows)]
                let transient = matches!(error.error.raw_os_error(), Some(5 | 32 | 33));
                #[cfg(not(windows))]
                let transient = false;
                if !transient || std::time::Instant::now() >= deadline {
                    return Err(error.to_string());
                }
                file = error.file;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
const MAX_STATE_BYTES: usize = 64 * 1024 * 1024;
static RECOVERY: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
pub fn recovery_notices() -> Vec<String> {
    std::mem::take(&mut *RECOVERY.lock().unwrap())
}
fn backup(path: &Path) -> std::path::PathBuf {
    path.with_extension("json.bak")
}
fn bounded_read(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut data = Vec::new();
    file.take((MAX_STATE_BYTES + 1) as u64)
        .read_to_end(&mut data)?;
    if data.len() > MAX_STATE_BYTES {
        return Err(std::io::Error::other(
            "Application state exceeds its size limit",
        ));
    }
    Ok(data)
}
pub fn atomic_queue(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_STATE_BYTES {
        return Err("Application state exceeds its size limit".into());
    }
    if let Ok(previous) = bounded_read(path) {
        if previous == bytes {
            return Ok(());
        }
        if serde_json::from_slice::<serde_json::Value>(&previous).is_ok() {
            atomic_private(&backup(path), &previous)?;
        }
    }
    atomic_private(path, bytes)
}
fn quarantine(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let name = path
        .file_name()
        .ok_or("Missing state filename")?
        .to_string_lossy();
    let target = path.with_file_name(format!("{name}.corrupt-{:016x}", rand::random::<u64>()));
    std::fs::rename(path, target).map_err(|e| e.to_string())
}
pub fn load_queue<T: serde::de::DeserializeOwned + Default>(
    path: &Path,
    label: &str,
) -> Result<T, String> {
    match bounded_read(path) {
        Ok(bytes) => {
            if let Ok(value) = serde_json::from_slice::<T>(&bytes) {
                return Ok(value);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !backup(path).exists() => {
            return Ok(T::default())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Err(e.to_string()),
        Err(_) => {}
    }
    if let Ok(bytes) = bounded_read(&backup(path)) {
        if let Ok(value) = serde_json::from_slice::<T>(&bytes) {
            quarantine(path)?;
            atomic_private(path, &bytes)?;
            RECOVERY.lock().unwrap().push(format!("{label}: se ha recuperado la última copia válida de la cola. Los archivos descargados se conservan."));
            return Ok(value);
        }
    }
    quarantine(path)?;
    quarantine(&backup(path))?;
    RECOVERY.lock().unwrap().push(format!("{label}: no se pudo recuperar la cola. Se ha conservado una copia de los datos dañados y no se han borrado archivos descargados."));
    Ok(T::default())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_keeps_complete_state_with_concurrent_readers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        atomic_private(&path, b"{\"n\":0}").unwrap();
        std::thread::scope(|scope| {
            let reader = scope.spawn(|| {
                for _ in 0..500 {
                    let bytes = std::fs::read(&path).unwrap();
                    assert!(serde_json::from_slice::<serde_json::Value>(&bytes).is_ok());
                }
            });
            for n in 1..25 {
                atomic_private(
                    &path,
                    serde_json::to_string(&serde_json::json!({"n":n}))
                        .unwrap()
                        .as_bytes(),
                )
                .unwrap();
            }
            reader.join().unwrap();
        });
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(path).unwrap()).unwrap()
                ["n"],
            24
        );
    }
    #[cfg(unix)]
    #[test]
    fn private_permissions_and_replacing_a_link_preserve_its_target() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("private");
        private_dir(&root).unwrap();
        let unrelated = dir.path().join("unrelated");
        std::fs::write(&unrelated, b"keep").unwrap();
        let path = root.join("state.json");
        symlink(&unrelated, &path).unwrap();
        atomic_private(&path, b"new").unwrap();
        assert_eq!(std::fs::read(unrelated).unwrap(), b"keep");
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(root).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn patched_variant_string_iteration_survives_optimization() {
        use glib::variant::ToVariant;
        let variant = vec!["first", "middle", "last"].to_variant();
        assert_eq!(
            variant.array_iter_str().unwrap().collect::<Vec<_>>(),
            vec!["first", "middle", "last"]
        );
        assert_eq!(variant.array_iter_str().unwrap().last(), Some("last"));
        let mut values = variant.array_iter_str().unwrap();
        assert_eq!(values.next_back(), Some("last"));
        assert_eq!(values.nth(1), Some("middle"));
    }
    #[test]
    fn corrupted_queue_recovers_valid_checkpoint_and_preserves_damaged_data() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("downloads.json");
        atomic_queue(&file, b"[1]").unwrap();
        atomic_queue(&file, b"[1,2]").unwrap();
        std::fs::write(&file, b"{").unwrap();
        assert_eq!(load_queue::<Vec<u32>>(&file, "Test").unwrap(), vec![1]);
        assert!(std::fs::read_dir(tmp.path()).unwrap().any(|p| p
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".corrupt-")));
        assert_eq!(
            serde_json::from_slice::<Vec<u32>>(&std::fs::read(file).unwrap()).unwrap(),
            vec![1]
        );
    }
    #[test]
    fn corrupt_primary_and_backup_do_not_prevent_startup_or_delete_payloads() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("downloads.json");
        std::fs::write(&file, b"bad").unwrap();
        std::fs::write(backup(&file), b"bad backup").unwrap();
        let payload = tmp.path().join("keep.bin");
        std::fs::write(&payload, b"keep").unwrap();
        assert!(load_queue::<Vec<u32>>(&file, "Test").unwrap().is_empty());
        assert_eq!(std::fs::read(payload).unwrap(), b"keep");
        assert_eq!(
            std::fs::read_dir(tmp.path())
                .unwrap()
                .filter(|p| p
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains(".corrupt-"))
                .count(),
            2
        );
    }
}
