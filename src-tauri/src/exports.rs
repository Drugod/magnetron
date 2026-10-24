use crate::model::Torrent;
pub fn encode(items: &[Torrent], format: &str) -> Result<String, String> {
    if items.len() > 10000 {
        return Err("Máximo 10000 resultados por exportación".into());
    }
    let result = match format {
        "json" => serde_json::to_string_pretty(items).map_err(|e| e.to_string())?,
        "links" => items
            .iter()
            .map(|t| t.ed2k_uri.as_deref().unwrap_or(&t.magnet_uri))
            .collect::<Vec<_>>()
            .join("\r\n"),
        "csv" => {
            fn cell(s: &str) -> String {
                let guarded = if s.trim_start().starts_with(['=', '+', '-', '@']) {
                    format!("'{s}")
                } else {
                    s.to_owned()
                };
                format!("\"{}\"", guarded.replace('"', "\"\""))
            }
            let mut out = String::from("name,network,size_bytes,link\r\n");
            for t in items {
                out.push_str(&format!(
                    "{},{},{},{}\r\n",
                    cell(&t.title),
                    cell(&t.network),
                    t.size_bytes.map_or(String::new(), |n| n.to_string()),
                    cell(t.ed2k_uri.as_deref().unwrap_or(&t.magnet_uri))
                ));
            }
            out
        }
        _ => return Err("Formato de exportación inválido".into()),
    };
    if result.len() > 32 * 1024 * 1024 {
        return Err("Exportación demasiado grande".into());
    }
    Ok(result)
}
#[tauri::command]
pub async fn export_results(
    items: Vec<Torrent>,
    format: String,
    language: Option<String>,
) -> Result<bool, String> {
    let text = encode(&items, &format)?;
    let extension = match format.as_str() {
        "json" => "json",
        "csv" => "csv",
        _ => "txt",
    };
    tauri::async_runtime::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_title(if language.as_deref() == Some("en") {
                "Export results"
            } else {
                "Exportar resultados"
            })
            .set_file_name(format!("MAGNETRON.{extension}"))
            .add_filter(extension, &[extension])
            .save_file();
        if let Some(path) = path {
            std::fs::write(path, text).map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
