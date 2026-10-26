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
    file.persist(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
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
}
