//! Untrusted browser/clipboard links enter through the same bounded validator.
use serde::Serialize;
use std::sync::Mutex;
use tauri::Emitter;
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());
#[derive(Serialize)]
pub struct Link {
    pub uri: String,
    pub name: String,
}
pub fn parse(text: &str) -> Result<Vec<Link>, String> {
    if text.len() > 1_000_000 {
        return Err("Pega como máximo 100 enlaces".into());
    }
    let mut links = vec![];
    for line in text.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if links.len() >= 100 {
            return Err("Pega como máximo 100 enlaces".into());
        }
        let link = if line.starts_with("ed2k://") {
            // AICH/source hints from websites are not commands and are not forwarded.
            let parts: Vec<_> = line.split('|').collect();
            if parts.len() < 6 || parts[1] != "file" || parts.last() != Some(&"/") {
                return Err("Se necesita un enlace eD2k de archivo".into());
            }
            for extra in &parts[5..parts.len() - 1] {
                if !extra.starts_with("h=")
                    || !extra[2..].bytes().all(|c| c.is_ascii_alphanumeric())
                {
                    return Err("Extensión de enlace eD2k no admitida".into());
                }
            }
            let uri = crate::ed2k::validate_link(&format!("{}|/", parts[..5].join("|")))?;
            let encoded = uri.split('|').nth(2).unwrap();
            let parsed = url::Url::parse(&format!(
                "https://local/?name={}",
                encoded.replace('+', "%2B")
            ))
            .map_err(|e| e.to_string())?;
            let name = parsed.query_pairs().next().unwrap().1.into_owned();
            Link { uri, name }
        } else if line.starts_with("magnet:") {
            let uri = crate::model::validate_magnet(line)?;
            let parsed = url::Url::parse(&uri).map_err(|e| e.to_string())?;
            let name = parsed
                .query_pairs()
                .find(|(k, _)| k == "dn")
                .map(|(_, v)| v.into_owned())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "Torrent".into());
            Link { uri, name }
        } else if line.starts_with("https://") {
            let (_, uri) = crate::video::identity(line)?;
            Link {
                uri,
                name: "YouTube".into(),
            }
        } else {
            return Err("Pega enlaces eD2k, magnet o YouTube, uno por línea".into());
        };
        if !links.iter().any(|old: &Link| old.uri == link.uri) {
            links.push(link)
        }
    }
    if links.is_empty() {
        return Err("Pega un enlace eD2k o magnet".into());
    }
    Ok(links)
}
pub fn enqueue(app: &tauri::AppHandle, args: impl IntoIterator<Item = String>) {
    let mut pending = PENDING.lock().unwrap();
    for arg in args {
        if pending.len() < 100
            && arg.len() <= 16384
            && (arg.starts_with("ed2k://") || arg.starts_with("magnet:"))
        {
            pending.push(arg)
        }
    }
    drop(pending);
    let _ = app.emit("download-links-ready", ());
}
#[tauri::command]
pub fn take_download_links() -> Vec<String> {
    std::mem::take(&mut *PENDING.lock().unwrap())
}
#[tauri::command]
pub fn parse_download_links(text: String) -> Result<Vec<Link>, String> {
    parse(&text)
}
#[tauri::command]
pub fn register_link_handlers(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_deep_link::DeepLinkExt;
    #[cfg(not(target_os = "macos"))]
    {
        app.deep_link().register_all().map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pasted_and_browser_links_are_strict() {
        let uri = format!("ed2k://|file|Mi%20fichero%2B.pdf|123|{}|/", "a".repeat(32));
        let rows = parse(&format!("{uri}\n{uri}")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Mi fichero+.pdf");
        let aich = uri.replace("|/", "|h=ABC123|/");
        assert_eq!(parse(&aich).unwrap()[0].uri, uri);
        assert!(parse("ed2k://|server|1.2.3.4|4661|/").is_err());
        assert!(parse("https://example.com/file").is_err());
        assert!(parse(&format!("{uri}\nmalformed")).is_err());
        let magnet = format!("magnet:?xt=urn:btih:{}&dn=Hola%20mundo", "a".repeat(40));
        assert_eq!(parse(&magnet).unwrap()[0].name, "Hola mundo");
        assert!(parse(&vec![uri; 101].join("\n")).is_ok()); // duplicates count only once
        let many = (0..101)
            .map(|i| format!("ed2k://|file|{i}.bin|123|{}|/", "a".repeat(32)))
            .collect::<Vec<_>>();
        assert!(parse(&many.join("\n")).is_err());
    }
}
