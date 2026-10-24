//! Opt-in release channel: trusted build-time key, detached Ed25519 signature,
//! bounded HTTPS download and digest verification before staging an installer.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::Manager;
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
#[derive(Serialize)]
pub struct Status {
    configured: bool,
    ready: bool,
    version: Option<String>,
}
fn channel() -> Option<(&'static str, &'static str)> {
    if !cfg!(windows) {
        return None;
    }
    Some((
        option_env!("MAGNETRON_UPDATE_URL")?,
        option_env!("MAGNETRON_UPDATE_PUBLIC_KEY")?,
    ))
}
fn version(s: &str) -> Result<Vec<u32>, String> {
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
    let bytes = std::fs::read(root.join("manifest.json")).map_err(|e| e.to_string())?;
    let release = verify(&bytes, key)?;
    if version(&release.version)? <= version(env!("CARGO_PKG_VERSION"))? {
        return Err("La versión ya está instalada".into());
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
    static CONTROL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = CONTROL.lock().await;
    let Some((url, key)) = channel() else {
        return Ok(Status {
            configured: false,
            ready: false,
            version: None,
        });
    };
    let root = root(&app)?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(120))
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
    if !staged(&root, key).is_ok_and(|s| s.version == release.version) {
        let bytes = get(&client, &release.url, release.size as usize).await?;
        if bytes.len() as u64 != release.size
            || hex::encode(Sha256::digest(&bytes)) != release.sha256.to_lowercase()
        {
            return Err("El instalador no coincide con la firma".into());
        }
        std::fs::write(root.join("installer.new"), bytes).map_err(|e| e.to_string())?;
        std::fs::rename(root.join("installer.new"), root.join("installer.exe"))
            .map_err(|e| e.to_string())?;
        std::fs::write(root.join("manifest.json"), manifest).map_err(|e| e.to_string())?;
    }
    Ok(Status {
        configured: true,
        ready: true,
        version: Some(release.version),
    })
}
#[tauri::command]
pub fn install_app_update(app: tauri::AppHandle) -> Result<(), String> {
    let (_, key) = channel().ok_or("Canal de publicación pendiente")?;
    let root = root(&app)?;
    staged(&root, key)?;
    crate::tray::exit_for_update(app, root.join("installer.exe"));
    Ok(())
}
pub fn configured() -> bool {
    channel().is_some()
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
            url: "https://example.com/setup.exe".into(),
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
}
