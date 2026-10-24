//! Official release staging: bounded downloads, published digest, confined extraction,
//! smoke test and atomic activation on the next launch. The bundled engine is retained.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[derive(Serialize, Deserialize, Default)]
struct State {
    checked: u64,
    version: String,
    previous: String,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn state(root: &Path) -> State {
    std::fs::read(root.join("state.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
fn valid_version(v: &str) -> bool {
    v.starts_with('v')
        && v.len() < 40
        && v[1..].split('.').count() == 3
        && v[1..].split('.').all(|n| {
            !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) && n.parse::<u32>().is_ok()
        })
}
fn version_parts(v: &str) -> Vec<u32> {
    v.trim_start_matches('v')
        .split('.')
        .filter_map(|n| n.parse().ok())
        .collect()
}
pub fn selected(data: &Path) -> Option<PathBuf> {
    let root = data.join("updates");
    let s = state(&root);
    if !valid_version(&s.version) {
        return None;
    }
    let exe = root
        .join(s.version)
        .join("Jackett")
        .join(crate::platform::jackett_name());
    exe.is_file().then_some(exe)
}
pub fn reject(data: &Path) {
    let root = data.join("updates");
    let mut s = state(&root);
    s.version = std::mem::take(&mut s.previous);
    let _ = save(&root, &s);
}
fn save(root: &Path, s: &State) -> Result<(), String> {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(s).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(root.join("state.json"))
        .map_err(|e| e.to_string())?;
    Ok(())
}
async fn get(client: &reqwest::Client, url: &str, max: usize) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > max as u64) {
        return Err("Actualización demasiado grande".into());
    }
    let mut bytes = vec![];
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > max {
            return Err("Actualización demasiado grande".into());
        }
        bytes.extend(chunk)
    }
    Ok(bytes)
}
fn extract(bytes: &[u8], root: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        extract_zip(bytes, root)
    }
    #[cfg(not(windows))]
    {
        extract_tar(bytes, root)
    }
}
#[cfg(windows)]
fn extract_zip(bytes: &[u8], root: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if zip.len() > 10000 {
        return Err("Demasiados archivos en la actualización".into());
    }
    let mut size = 0u64;
    let mut seen = std::collections::HashSet::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        size = size.checked_add(file.size()).ok_or("Tamaño inválido")?;
        if size > 600 * 1024 * 1024 {
            return Err("Actualización descomprimida demasiado grande".into());
        }
        let relative = file
            .enclosed_name()
            .ok_or("Ruta fuera de la actualización")?;
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
            || relative.to_string_lossy().contains([':', '\\'])
            || file.is_symlink()
        {
            return Err("Ruta inválida en la actualización".into());
        }
        if !seen.insert(relative.to_string_lossy().to_lowercase()) {
            return Err("Ruta duplicada en la actualización".into());
        }
        let target = root.join(&relative);
        if file.is_dir() {
            std::fs::create_dir_all(target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?
        }
        let mut output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(target)
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut file, &mut output).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(not(windows))]
fn extract_tar(bytes: &[u8], root: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    let mut size = 0u64;
    let mut count = 0;
    let mut seen = std::collections::HashSet::new();
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        count += 1;
        if count > 10000 {
            return Err("Too many update entries".into());
        }
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err("Links and special update entries are not allowed".into());
        }
        let original = entry.path().map_err(|e| e.to_string())?;
        let mut relative = PathBuf::new();
        for component in original.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::Normal(c) => relative.push(c),
                _ => return Err("Unsafe update path".into()),
            }
        }
        if relative.as_os_str().is_empty() {
            continue;
        }
        if relative.to_string_lossy().contains([':', '\\'])
            || !seen.insert(relative.to_string_lossy().to_lowercase())
        {
            return Err("Unsafe or duplicate update path".into());
        }
        size = size
            .checked_add(entry.size())
            .ok_or("Invalid update size")?;
        if size > 600 * 1024 * 1024 {
            return Err("Update too large".into());
        }
        let target = root.join(relative);
        if kind.is_dir() {
            std::fs::create_dir_all(target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&target)
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        std::fs::set_permissions(
            target,
            std::fs::Permissions::from_mode(if entry.header().mode().unwrap_or(0) & 0o111 != 0 {
                0o755
            } else {
                0o644
            }),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub async fn check(data: PathBuf) -> Result<(), String> {
    let root = data.join("updates");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let mut s = state(&root);
    if now().saturating_sub(s.checked) < 86400 {
        return Ok(());
    }
    // Record attempts too: offline sessions never generate a retry storm.
    s.checked = now();
    save(&root, &s)?;
    let client = reqwest::Client::builder()
        .user_agent("MAGNETRON-source-updater")
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            let host = url.host_str().unwrap_or("");
            if attempt.previous().len() < 5
                && url.scheme() == "https"
                && (host == "github.com"
                    || host == "api.github.com"
                    || host == "codeload.github.com"
                    || host.ends_with(".githubusercontent.com"))
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|e| e.to_string())?;
    let release: serde_json::Value = serde_json::from_slice(
        &get(
            &client,
            "https://api.github.com/repos/Jackett/Jackett/releases/latest",
            2 * 1024 * 1024,
        )
        .await?,
    )
    .map_err(|e| e.to_string())?;
    let version = release["tag_name"]
        .as_str()
        .filter(|v| valid_version(v))
        .ok_or("Versión oficial inválida")?
        .to_string();
    if release["draft"].as_bool() != Some(false) || release["prerelease"].as_bool() != Some(false) {
        return Err("Se requiere una versión estable".into());
    }
    if version_parts(&version)
        <= version_parts(if valid_version(&s.version) {
            &s.version
        } else {
            "v0.24.2900"
        })
    {
        return Ok(());
    }
    let asset = release["assets"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|a| a["name"] == crate::platform::jackett_asset())
        })
        .ok_or("Falta el paquete de esta plataforma")?;
    let digest = asset["digest"]
        .as_str()
        .and_then(|v| v.strip_prefix("sha256:"))
        .filter(|v| v.len() == 64 && v.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or("La versión no publica una huella SHA-256")?;
    let url = format!(
        "https://github.com/Jackett/Jackett/releases/download/{version}/{}",
        crate::platform::jackett_asset()
    );
    let bytes = get(&client, &url, 150 * 1024 * 1024).await?;
    if !hex::encode(Sha256::digest(&bytes)).eq_ignore_ascii_case(digest) {
        return Err("La huella de la actualización no coincide".into());
    }
    let stage = tempfile::Builder::new()
        .prefix("stage-")
        .tempdir_in(&root)
        .map_err(|e| e.to_string())?;
    let staged = stage.path().to_owned();
    tokio::task::spawn_blocking(move || extract(&bytes, &staged))
        .await
        .map_err(|e| e.to_string())??;
    // Keep the corresponding upstream source alongside the runtime for redistribution.
    let source = get(
        &client,
        &format!("https://codeload.github.com/Jackett/Jackett/zip/refs/tags/{version}"),
        80 * 1024 * 1024,
    )
    .await?;
    zip::ZipArchive::new(std::io::Cursor::new(&source))
        .map_err(|_| "Código fuente oficial inválido")?;
    std::fs::write(stage.path().join("Jackett-source.zip"), source).map_err(|e| e.to_string())?;
    let exe = stage
        .path()
        .join("Jackett")
        .join(crate::platform::jackett_name());
    if !exe.is_file() {
        return Err("Paquete oficial incompleto".into());
    }
    let probe = tempfile::Builder::new()
        .prefix("probe-")
        .tempdir_in(&root)
        .map_err(|e| e.to_string())?;
    let engine = crate::engine::Engine::new(exe, probe.path().to_owned());
    let catalog = engine.catalog().await;
    drop(engine);
    if catalog?.is_empty() {
        return Err("El motor actualizado no tiene fuentes públicas".into());
    }
    let final_path = root.join(&version);
    if final_path.exists() {
        return Err("Ya existe una preparación de esta versión".into());
    }
    std::fs::rename(stage.path(), &final_path).map_err(|e| e.to_string())?;
    s.previous = s.version;
    s.version = version;
    save(&root, &s)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Descarga y prueba la versión estable oficial de Jackett"]
    async fn live_official_update() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/update-tests")
            .join(format!("{:x}", rand::random::<u64>()));
        std::fs::create_dir_all(&root).unwrap();
        check(root.clone()).await.unwrap();
        assert!(selected(&root).is_some());
        assert!(root
            .join("updates")
            .join(state(&root.join("updates")).version)
            .join("Jackett-source.zip")
            .is_file());
        // A candidate which no longer starts must never prevent use of the bundled engine.
        std::fs::write(selected(&root).unwrap(), b"invalid executable").unwrap();
        let bundled = crate::platform::dev_jackett();
        let fallback = crate::engine::Engine::new(bundled, root.clone());
        assert!(!fallback.catalog().await.unwrap().is_empty());
        drop(fallback);
        assert!(selected(&root).is_none());
    }
    #[test]
    fn versions_and_archives_are_confined() {
        assert!(valid_version("v0.24.2900"));
        for v in ["../v1.2.3", "v1.2.3/evil", "v1..2"] {
            assert!(!valid_version(v))
        }
        let temp = tempfile::tempdir().unwrap();
        let mut output = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        output
            .start_file("../evil.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        let bytes = output.finish().unwrap().into_inner();
        assert!(extract(&bytes, temp.path()).is_err());
    }
    #[cfg(not(windows))]
    #[test]
    fn native_archive_preserves_execute_bits_and_rejects_links() {
        use std::os::unix::fs::PermissionsExt;
        fn archive(link: bool) -> Vec<u8> {
            let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            let mut builder = tar::Builder::new(gz);
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o755);
            if link {
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_size(0);
                header.set_link_name("../../outside").unwrap();
            } else {
                header.set_size(4);
            }
            header.set_cksum();
            builder
                .append_data(
                    &mut header,
                    "Jackett/jackett",
                    if link { &b""[..] } else { &b"test"[..] },
                )
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap()
        }
        let temp = tempfile::tempdir().unwrap();
        extract(&archive(false), temp.path()).unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("Jackett/jackett")).unwrap(),
            b"test"
        );
        assert_ne!(
            std::fs::metadata(temp.path().join("Jackett/jackett"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
        let other = tempfile::tempdir().unwrap();
        assert!(extract(&archive(true), other.path()).is_err());
    }
}
