//! Opt-in release channel: trusted build-time key, detached Ed25519 signature,
//! bounded HTTPS download and digest verification before staging an installer.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};
static CONTROL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[derive(Deserialize, Serialize)]
struct Envelope {
    payload: String,
    signature: String,
}
#[derive(Deserialize, Serialize)]
struct Release {
    version: String,
    url: String,
    sha256: String,
    size: u64,
}
#[derive(Clone, Serialize)]
pub struct Status {
    configured: bool,
    ready: bool,
    version: Option<String>,
}
fn platform() -> &'static str {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macos-arm64"
        } else {
            "macos-x64"
        }
    } else if cfg!(target_os = "linux") {
        if std::env::var_os("APPIMAGE").is_some() {
            "linux-appimage"
        } else {
            "linux-deb"
        }
    } else {
        "windows"
    }
}
fn official_url(version: &str) -> String {
    let suffix = match platform() {
        "linux-appimage" => "amd64.AppImage",
        "linux-deb" => "amd64.deb",
        "macos-arm64" => "macos-arm64.app.zip",
        "macos-x64" => "macos-x64.app.zip",
        _ => "x64-setup.exe",
    };
    format!("https://github.com/Drugod/magnetron/releases/download/v{version}/MAGNETRON_{version}_{suffix}")
}
fn channel() -> Option<(&'static str, &'static str)> {
    let url = match platform() {
        "linux-appimage" => "https://github.com/Drugod/magnetron/releases/latest/download/linux-appimage-update.json",
        "linux-deb" => "https://github.com/Drugod/magnetron/releases/latest/download/linux-deb-update.json",
        "macos-arm64" => "https://github.com/Drugod/magnetron/releases/latest/download/macos-arm64-update.json",
        "macos-x64" => "https://github.com/Drugod/magnetron/releases/latest/download/macos-x64-update.json",
        _ if cfg!(all(windows, target_arch = "x86_64")) => "https://github.com/Drugod/magnetron/releases/latest/download/windows-update.json",
        _ => return None,
    };
    Some((url, include_str!("../update-public-key.txt").trim()))
}
fn version(s: &str) -> Result<Vec<u32>, String> {
    if s.len() > 32
        || s.split('.').any(|p| {
            p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
        })
    {
        return Err("Versión inválida".into());
    }
    let parts = s
        .split('.')
        .map(|p| p.parse::<u32>().map_err(|_| "Versión inválida".to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if parts.len() != 3 {
        return Err("Versión inválida".into());
    }
    Ok(parts)
}
fn https(value: &str) -> Result<(), String> {
    let url = url::Url::parse(value).map_err(|e| e.to_string())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("La actualización requiere HTTPS".into());
    }
    Ok(())
}
fn verify(bytes: &[u8], public: &str) -> Result<Release, String> {
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let key = hex::decode(public).map_err(|_| "Clave pública inválida")?;
    let signature = hex::decode(envelope.signature).map_err(|_| "Firma inválida")?;
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key)
        .verify(envelope.payload.as_bytes(), &signature)
        .map_err(|_| "Firma de actualización inválida")?;
    let release: Release = serde_json::from_str(&envelope.payload).map_err(|e| e.to_string())?;
    version(&release.version)?;
    https(&release.url)?;
    let expected = official_url(&release.version);
    if release.url != expected {
        return Err("El paquete no pertenece al canal oficial de este sistema".into());
    }
    if release.size == 0
        || release.size > 512 * 1024 * 1024
        || release.sha256.len() != 64
        || !release.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Manifiesto inválido".into());
    }
    Ok(release)
}
async fn get(client: &reqwest::Client, url: &str, max: usize) -> Result<Vec<u8>, String> {
    https(url)?;
    let mut response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(if max <= 65536 {
            30
        } else {
            900
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    https(response.url().as_str())?;
    if response.content_length().is_some_and(|n| n > max as u64) {
        return Err("Actualización demasiado grande".into());
    }
    let mut data = vec![];
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if data.len() + chunk.len() > max {
            return Err("Actualización demasiado grande".into());
        }
        data.extend(chunk)
    }
    Ok(data)
}
fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("app-updates"))
}
fn staged(root: &Path, key: &str) -> Result<Release, String> {
    validate_staged(root, key, env!("CARGO_PKG_VERSION"))
}
fn validate_staged(root: &Path, key: &str, installed: &str) -> Result<Release, String> {
    if std::fs::metadata(root.join("manifest.json"))
        .map_err(|e| e.to_string())?
        .len()
        > 65536
    {
        return Err("Manifiesto inválido".into());
    }
    let bytes = std::fs::read(root.join("manifest.json")).map_err(|e| e.to_string())?;
    let release = verify(&bytes, key)?;
    if version(&release.version)? <= version(installed)? {
        return Err("La versión ya está instalada".into());
    }
    if std::fs::metadata(root.join("installer.exe"))
        .map_err(|e| e.to_string())?
        .len()
        != release.size
    {
        return Err("El instalador no coincide con la firma".into());
    }
    let installer = std::fs::read(root.join("installer.exe")).map_err(|e| e.to_string())?;
    if installer.len() as u64 != release.size
        || hex::encode(Sha256::digest(&installer)) != release.sha256.to_lowercase()
    {
        return Err("El instalador no coincide con la firma".into());
    }
    Ok(release)
}
#[tauri::command]
pub async fn check_app_update(app: tauri::AppHandle) -> Result<Status, String> {
    let _guard = CONTROL.lock().await;
    let Some((url, key)) = channel() else {
        return Ok(Status {
            configured: false,
            ready: false,
            version: None,
        });
    };
    let root = root(&app)?;
    crate::secure_fs::private_dir(&root)?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(900))
        .build()
        .map_err(|e| e.to_string())?;
    let manifest = get(&client, url, 65536).await?;
    let release = verify(&manifest, key)?;
    if version(&release.version)? <= version(env!("CARGO_PKG_VERSION"))? {
        return Ok(Status {
            configured: true,
            ready: false,
            version: None,
        });
    }
    if let Ok(cached) = staged(&root, key) {
        if version(&cached.version)? > version(&release.version)? {
            return Err("Se ha rechazado una actualización anterior a la ya preparada".into());
        }
    }
    if !staged(&root, key).is_ok_and(|s| s.version == release.version && s.sha256 == release.sha256)
    {
        let bytes = get(&client, &release.url, release.size as usize).await?;
        if bytes.len() as u64 != release.size
            || hex::encode(Sha256::digest(&bytes)) != release.sha256.to_lowercase()
        {
            return Err("El instalador no coincide con la firma".into());
        }
        crate::secure_fs::atomic_private(&root.join("installer.exe"), &bytes)?;
        crate::secure_fs::atomic_private(&root.join("manifest.json"), &manifest)?;
    }
    let status = Status {
        configured: true,
        ready: true,
        version: Some(release.version),
    };
    let _ = app.emit("app-update-ready", &status);
    Ok(status)
}
#[tauri::command]
pub async fn install_app_update(app: tauri::AppHandle) -> Result<(), String> {
    let _guard = CONTROL.lock().await;
    let (_, key) = channel().ok_or("Canal de publicación pendiente")?;
    let root = root(&app)?;
    staged(&root, key)?;
    crate::tray::exit_for_update(app, root.join("installer.exe"));
    Ok(())
}
pub fn configured() -> bool {
    channel().is_some()
}
#[cfg(unix)]
pub fn launch_native(app: &tauri::AppHandle, installer: &Path) -> Result<(), String> {
    let root = root(app)?;
    if installer != root.join("installer.exe") {
        return Err("Ruta de actualización inválida".into());
    }
    staged(&root, include_str!("../update-public-key.txt").trim())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = if cfg!(target_os = "macos") {
        exe.ancestors()
            .find(|p| p.file_name().is_some_and(|n| n == "MAGNETRON.app"))
            .ok_or("Instala MAGNETRON en Aplicaciones antes de actualizar")?
            .to_path_buf()
    } else if platform() == "linux-appimage" {
        let image = PathBuf::from(std::env::var_os("APPIMAGE").ok_or("Falta la ruta AppImage")?);
        let appdir = PathBuf::from(std::env::var_os("APPDIR").ok_or("Falta el montaje AppImage")?);
        if !exe.starts_with(appdir)
            || !image.is_absolute()
            || image.extension().is_none_or(|e| e != "AppImage")
        {
            return Err("La ejecución no pertenece a una AppImage válida".into());
        }
        image
    } else {
        if !Path::new("/usr/bin/pkexec").is_file() || !Path::new("/usr/bin/dpkg").is_file() {
            return Err(
                "Este sistema necesita PackageKit/polkit para instalar el paquete Debian".into(),
            );
        }
        exe.clone()
    };
    let mut runtime = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("v2");
    #[cfg(debug_assertions)]
    if !runtime.join("native_update.mjs").is_file() {
        runtime = crate::platform::dev_runtime();
    }
    let node = runtime.join("video-tools/node");
    if !node.is_file() {
        return Err("Falta el instalador integrado de actualizaciones".into());
    }
    let helper = root.join("native_update.mjs");
    crate::secure_fs::atomic_private(
        &helper,
        &std::fs::read(runtime.join("native_update.mjs")).map_err(|e| e.to_string())?,
    )?;
    let request = serde_json::json!({"root":root,"platform":platform(),"target":target,"installed":env!("CARGO_PKG_VERSION"),"pid":std::process::id(),"publicKey":include_str!("../update-public-key.txt").trim()});
    let request_file = root.join("install-request.json");
    crate::secure_fs::atomic_private(
        &request_file,
        &serde_json::to_vec(&request).map_err(|e| e.to_string())?,
    )?;
    use std::os::unix::process::CommandExt;
    std::process::Command::new(node)
        .arg(helper)
        .arg(request_file)
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn confirm_startup(app: &tauri::AppHandle) {
    for arg in std::env::args().skip(1) {
        if let Some(token) = arg.strip_prefix("--magnetron-update-health=") {
            if token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit()) {
                if let Ok(root) = root(app) {
                    let _ = crate::secure_fs::atomic_private(
                        &root.join(format!("health-{token}")),
                        env!("CARGO_PKG_VERSION").as_bytes(),
                    );
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair;
    #[test]
    fn rejects_tamper_and_rollback() {
        let rng = ring::rand::SystemRandom::new();
        let key = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let key = ring::signature::Ed25519KeyPair::from_pkcs8(key.as_ref()).unwrap();
        let payload = serde_json::to_string(&Release {
            version: "0.8.1".into(),
            url: official_url("0.8.1"),
            sha256: "a".repeat(64),
            size: 123,
        })
        .unwrap();
        let sig = key.sign(payload.as_bytes());
        let mut envelope = Envelope {
            payload,
            signature: hex::encode(sig.as_ref()),
        };
        let public = hex::encode(key.public_key().as_ref());
        assert!(verify(&serde_json::to_vec(&envelope).unwrap(), &public).is_ok());
        envelope.payload.push(' ');
        assert!(verify(&serde_json::to_vec(&envelope).unwrap(), &public).is_err());
        assert!(https("http://example.com").is_err());
        assert!(version("0.8.0").unwrap() > version("0.7.0").unwrap());
    }
    #[test]
    fn rejects_wrong_key_origin_truncation_and_staged_tampering() {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let key = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let public = hex::encode(key.public_key().as_ref());
        let installer = b"synthetic installer fixture";
        let mut release = Release {
            version: "999.0.0".into(),
            url: official_url("999.0.0"),
            sha256: hex::encode(Sha256::digest(installer)),
            size: installer.len() as u64,
        };
        let sign = |r: &Release| {
            let payload = serde_json::to_string(r).unwrap();
            let signature = hex::encode(key.sign(payload.as_bytes()).as_ref());
            serde_json::to_vec(&Envelope { payload, signature }).unwrap()
        };
        let manifest = sign(&release);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("manifest.json"), &manifest).unwrap();
        std::fs::write(dir.path().join("installer.exe"), installer).unwrap();
        assert!(staged(dir.path(), &public).is_ok());
        std::fs::write(dir.path().join("installer.exe"), b"tampered").unwrap();
        assert!(staged(dir.path(), &public).is_err());
        assert!(verify(&manifest, &"00".repeat(32)).is_err());
        assert!(verify(&manifest[..manifest.len() / 2], &public).is_err());
        release.url = "https://example.com/installer.exe".into();
        assert!(verify(&sign(&release), &public).is_err());
        for invalid in ["0.8", "0.8.1.2", "0.08.1", "0.8.-1", "0.8.+1"] {
            assert!(version(invalid).is_err());
        }
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "Requires the signed installer artifact produced by the release workflow"]
    fn validates_real_signed_release_before_installation() {
        let source = PathBuf::from(std::env::var("MAGNETRON_UPDATE_TEST_DIR").unwrap());
        let dir = tempfile::tempdir().unwrap();
        let manifest = std::fs::read(source.join("windows-update.json")).unwrap();
        let key = include_str!("../update-public-key.txt").trim();
        let release = verify(&manifest, key).unwrap();
        std::fs::copy(
            source.join(format!("MAGNETRON_{}_x64-setup.exe", release.version)),
            dir.path().join("installer.exe"),
        )
        .unwrap();
        std::fs::write(dir.path().join("manifest.json"), manifest).unwrap();
        assert!(validate_staged(dir.path(), key, "0.8.0").is_ok());
        assert!(validate_staged(dir.path(), key, &release.version).is_err());
        // Change one byte, preserving length: the digest must reject it.
        use std::io::{Read, Seek, SeekFrom, Write};
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir.path().join("installer.exe"))
            .unwrap();
        let mut byte = [0u8];
        f.read_exact(&mut byte).unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();
        f.write_all(&[byte[0] ^ 1]).unwrap();
        drop(f);
        assert!(validate_staged(dir.path(), key, "0.8.0").is_err());
    }
}
