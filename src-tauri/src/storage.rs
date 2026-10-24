use crate::model::Torrent;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub ubuntu: bool,
    pub debian: bool,
    pub archive: bool,
    pub ed2k: bool,
    pub kad: bool,
    pub auto_sources: bool,
    pub download_folder: String,
    pub download_limit_kib: u32,
    pub upload_limit_kib: u32,
    pub shared_folders: Vec<crate::sharing::Folder>,
    pub history: bool,
    pub concurrency: usize,
    pub timeout_seconds: u64,
    pub cache_hours: u64,
    pub indexers: Vec<String>,
    pub local_index: bool,
    pub index_max_records: u32,
    pub discovery_interval_seconds: u64,
    pub discovery_timeout_seconds: u64,
    pub index_max_mib: u32,
    pub discovery_enabled: bool,
    pub discovery_start_hour: u32,
    pub discovery_end_hour: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            ubuntu: true,
            debian: true,
            archive: true,
            ed2k: true,
            kad: true,
            auto_sources: true,
            download_folder: String::new(),
            download_limit_kib: 0,
            upload_limit_kib: 0,
            shared_folders: vec![],
            history: false,
            concurrency: 4,
            timeout_seconds: 60,
            cache_hours: 24,
            indexers: vec![
                "torrentscsv".into(),
                "knaben".into(),
                "nyaasi".into(),
                "animetosho-xyz".into(),
            ],
            local_index: true,
            index_max_records: 50000,
            discovery_interval_seconds: 5,
            discovery_timeout_seconds: 25,
            index_max_mib: 256,
            discovery_enabled: false,
            discovery_start_hour: 0,
            discovery_end_hour: 24,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.discovery_start_hour > 23
            || self.discovery_end_hour > 24
            || self.discovery_start_hour == self.discovery_end_hour
        {
            return Err("Horario del índice inválido".into());
        }
        if !(1000..=1000000).contains(&self.index_max_records)
            || !(2..=3600).contains(&self.discovery_interval_seconds)
            || !(5..=120).contains(&self.discovery_timeout_seconds)
            || !(32..=4096).contains(&self.index_max_mib)
        {
            return Err("Límites del índice fuera de rango".into());
        }
        if self.shared_folders.len() > 32 {
            return Err("Máximo 32 carpetas compartidas".into());
        }
        for folder in &self.shared_folders {
            if folder.path.is_empty()
                || folder.path.chars().any(char::is_control)
                || !std::path::Path::new(&folder.path).is_absolute()
            {
                return Err("Carpeta compartida inválida".into());
            }
        }
        for limit in [self.download_limit_kib, self.upload_limit_kib] {
            if limit != 0 && !(32..=1_048_576).contains(&limit) {
                return Err(
                    "La velocidad debe ser 0 (sin límite) o entre 32 y 1048576 KiB/s".into(),
                );
            }
        }
        if self.download_folder.len() > 32760 || self.download_folder.chars().any(char::is_control)
        {
            return Err("Carpeta de descarga inválida".into());
        }
        if self.indexers.iter().any(|id| {
            id.is_empty()
                || id.len() > 80
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        }) {
            return Err("Identificador de fuente inválido".into());
        }
        if !(1..=8).contains(&self.concurrency)
            || !(5..=120).contains(&self.timeout_seconds)
            || !(1..=720).contains(&self.cache_hours)
        {
            Err("Límites fuera de rango".into())
        } else {
            Ok(())
        }
    }
}
pub struct Storage {
    db: Connection,
}
impl Storage {
    pub fn new(path: &std::path::Path) -> Result<Self, String> {
        let db = Connection::open(path).map_err(|e| e.to_string())?;
        let version: u32 = db
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| e.to_string())?;
        if version > 1 {
            return Err(
                "La base de datos pertenece a una versión más reciente de MAGNETRON".into(),
            );
        }
        db.execute_batch(include_str!("../migrations/001.sql"))
            .map_err(|e| e.to_string())?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS favorites(id TEXT PRIMARY KEY,json TEXT NOT NULL,saved_at INTEGER NOT NULL);").map_err(|e|e.to_string())?;
        Ok(Self { db })
    }
    pub fn settings(&self) -> Result<Settings, String> {
        let mut stmt = self
            .db
            .prepare("SELECT value FROM settings WHERE key='preferences'")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        match rows.next().map_err(|e| e.to_string())? {
            Some(r) => serde_json::from_str(&r.get::<_, String>(0).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string()),
            None => Ok(Settings::default()),
        }
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO settings(key,value) VALUES ('preferences',?1)",
                [serde_json::to_string(settings).map_err(|e| e.to_string())?],
            )
            .map_err(|e| e.to_string())?;
        if !settings.history {
            self.db
                .execute("DELETE FROM search_history", [])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn remember(&self, query: &str) -> Result<(), String> {
        self.db
            .execute(
                "INSERT INTO search_history(query,searched_at) VALUES (?1,unixepoch())",
                [query],
            )
            .map_err(|e| e.to_string())?;
        self.db.execute("DELETE FROM search_history WHERE id NOT IN (SELECT id FROM search_history ORDER BY id DESC LIMIT 100)",[]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn history(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT query FROM search_history GROUP BY query ORDER BY max(id) DESC LIMIT 12",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn cache(&self, torrents: &[Torrent]) -> Result<(), String> {
        for incoming in torrents {
            let mut t = incoming.clone();
            let existing: Option<String> = self
                .db
                .query_row("SELECT json FROM torrents WHERE id=?1", [&t.id], |row| {
                    row.get(0)
                })
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(json) = existing {
                let old: Torrent = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                if t.files.is_empty() {
                    t.files = old.files;
                }
                for kind in old.content_types {
                    if !t.content_types.contains(&kind) {
                        t.content_types.push(kind);
                    }
                }
                if t.size_bytes.is_none() {
                    t.size_bytes = old.size_bytes;
                }
                if t.published_at.is_none() {
                    t.published_at = old.published_at;
                }
                for source in old.sources {
                    if !t.sources.contains(&source) {
                        t.sources.push(source);
                    }
                }
            }
            let searchable = format!("{} {}", t.title, t.files.join(" "));
            self.db.execute("INSERT INTO torrents(id,title,json,observed_at) VALUES (?1,?2,?3,unixepoch()) ON CONFLICT(id) DO UPDATE SET title=excluded.title,json=excluded.json,observed_at=excluded.observed_at",params![t.id,searchable,serde_json::to_string(&t).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
        }
        let cfg = self.settings()?;
        // Bound both record count and stored text. FTS and WAL have additional overhead.
        self.db.execute("DELETE FROM torrents WHERE id NOT IN (SELECT id FROM torrents ORDER BY observed_at DESC,id LIMIT ?1)",[cfg.index_max_records]).map_err(|e|e.to_string())?;
        loop {
            let bytes:u64=self.db.query_row("SELECT coalesce(sum(length(CAST(json AS BLOB))+length(CAST(title AS BLOB))),0) FROM torrents",[],|r|r.get(0)).map_err(|e|e.to_string())?;
            if bytes <= cfg.index_max_mib as u64 * 1024 * 1024 {
                break;
            }
            self.db.execute("DELETE FROM torrents WHERE id IN (SELECT id FROM torrents ORDER BY observed_at,id LIMIT max(1,(SELECT count(*)/10 FROM torrents)))",[]).map_err(|e|e.to_string())?;
        }
        Ok(())
    }
    pub fn favorites(&self) -> Result<Vec<Torrent>, String> {
        let mut stmt = self
            .db
            .prepare("SELECT json FROM favorites ORDER BY saved_at DESC,id LIMIT 10000")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.map(|r| {
            serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
        })
        .collect()
    }
    pub fn favorite(&self, torrent: &Torrent, enabled: bool) -> Result<(), String> {
        if torrent.network == "ed2k" {
            crate::ed2k::validate_link(torrent.ed2k_uri.as_deref().ok_or("Falta el enlace eD2k")?)?;
        } else {
            crate::model::validate_magnet(&torrent.magnet_uri)?;
        }
        let json = serde_json::to_string(torrent).map_err(|e| e.to_string())?;
        if json.len() > 4 * 1024 * 1024 {
            return Err("Favorito demasiado grande".into());
        }
        if enabled {
            let count: i64 = self
                .db
                .query_row(
                    "SELECT COUNT(*) FROM favorites WHERE id<>?1",
                    [&torrent.id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if count >= 10000 {
                return Err("Máximo 10000 favoritos".into());
            }
            self.db.execute("INSERT INTO favorites(id,json,saved_at) VALUES (?1,?2,unixepoch()) ON CONFLICT(id) DO UPDATE SET json=excluded.json",params![torrent.id,json]).map_err(|e|e.to_string())?;
        } else {
            self.db
                .execute("DELETE FROM favorites WHERE id=?1", [&torrent.id])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn search(&self, query: &str, settings: &Settings) -> Result<Vec<Torrent>, String> {
        let terms: Vec<_> = query
            .split_whitespace()
            .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
            .collect();
        if terms.is_empty() {
            return Ok(vec![]);
        }
        let mut stmt=self.db.prepare("SELECT t.json FROM torrent_fts f JOIN torrents t ON t.rowid=f.rowid WHERE torrent_fts MATCH ?1 AND (?3 OR t.observed_at >= unixepoch()-?2) ORDER BY rank LIMIT 300").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(
                params![
                    terms.join(" AND "),
                    settings.cache_hours * 3600,
                    settings.local_index
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| e.to_string())?;
        let mut result = vec![];
        for row in rows {
            let mut t: Torrent = serde_json::from_str(&row.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            t.sources.retain(|s| {
                (s == "ubuntu" && settings.ubuntu)
                    || (s == "debian" && settings.debian)
                    || (s == "archive" && settings.archive)
                    || (s == "dht" && settings.local_index)
                    || (s.starts_with("ed2k:") && settings.ed2k)
                    || (s == "kad" && settings.kad)
                    || s.strip_prefix("indexer:").is_some_and(|id| {
                        settings.auto_sources || settings.indexers.iter().any(|i| i == id)
                    })
            });
            if !t.sources.is_empty() {
                result.push(t);
            }
        }
        Ok(result)
    }
    pub fn clear(&self) -> Result<(), String> {
        self.db
            .execute_batch("DELETE FROM torrents; DELETE FROM search_history; VACUUM;")
            .map_err(|e| e.to_string())
    }
    pub fn clear_history(&self) -> Result<(), String> {
        self.db
            .execute("DELETE FROM search_history", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn known_hashes(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .db
            .prepare("SELECT id FROM torrents WHERE length(id)=40 AND id NOT GLOB '*[^0-9a-f]*' ORDER BY observed_at DESC LIMIT 50")
            .map_err(|e| e.to_string())?;
        let values = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        values
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn count(&self) -> Result<u64, String> {
        self.db
            .query_row("SELECT count(*) FROM torrents", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }
    pub fn find(&self, id: &str) -> Result<Option<Torrent>, String> {
        let json: Option<String> = self
            .db
            .query_row("SELECT json FROM torrents WHERE id=?1", [id], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        json.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
            .transpose()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_and_schema() {
        let s = Storage::new(std::path::Path::new(":memory:")).unwrap();
        assert!(!s.settings().unwrap().history);
        let mut cfg = Settings::default();
        cfg.history = true;
        s.save_settings(&cfg).unwrap();
        s.remember("ubuntu").unwrap();
        assert_eq!(s.history().unwrap(), vec!["ubuntu"]);
        s.clear().unwrap();
        assert!(s.history().unwrap().is_empty());
        assert!(s.search("\" OR *", &cfg).is_ok());
    }
    #[test]
    fn cache_updates_fts_and_respects_disabled_sources() {
        let s = Storage::new(std::path::Path::new(":memory:")).unwrap();
        let mut t = Torrent {
            id: "test".into(),
            title: "ubuntu desktop.iso".into(),
            infohash_v1: "a".repeat(40),
            magnet_uri: String::new(),
            size_bytes: None,
            published_at: None,
            seeds_reported: None,
            availability_status: "unknown".into(),
            availability_checked_at: None,
            sources: vec!["ubuntu".into()],
            files: vec![],
            content_types: vec![],
            network: "bittorrent".into(),
            ed2k_uri: None,
            complete_sources_reported: None,
            discovered_at: String::new(),
        };
        t.magnet_uri = format!("magnet:?xt=urn:btih:{}", t.infohash_v1);
        s.favorite(&t, true).unwrap();
        s.favorite(&t, true).unwrap();
        assert_eq!(s.favorites().unwrap().len(), 1);
        let csv = crate::exports::encode(
            &[Torrent {
                title: "=danger,\"quoted\"".into(),
                ..t.clone()
            }],
            "csv",
        )
        .unwrap();
        assert!(csv.contains("\"'=danger,\"\"quoted\"\"\""));
        assert!(crate::exports::encode(&[t.clone()], "links")
            .unwrap()
            .starts_with("magnet:"));
        s.clear().unwrap();
        assert_eq!(s.favorites().unwrap().len(), 1);
        s.favorite(&t, false).unwrap();
        assert!(s.favorites().unwrap().is_empty());
        s.cache(&[t.clone()]).unwrap();
        s.cache(&[t.clone()]).unwrap();
        let mut cfg = Settings::default();
        assert_eq!(s.search("ubuntu desktop", &cfg).unwrap().len(), 1);
        cfg.ubuntu = false;
        assert!(s.search("ubuntu", &cfg).unwrap().is_empty());
        cfg.ubuntu = true;
        t.title = "debian netinst.iso".into();
        s.cache(&[t]).unwrap();
        assert!(s.search("ubuntu", &cfg).unwrap().is_empty());
        assert_eq!(s.search("debian", &cfg).unwrap().len(), 1);
        s.db.execute("UPDATE torrents SET observed_at=0", [])
            .unwrap();
        cfg.local_index = false;
        assert!(s.search("debian", &cfg).unwrap().is_empty());
        s.remember("consulta de prueba").unwrap();
        s.clear_history().unwrap();
        assert!(s.history().unwrap().is_empty());
        assert_eq!(
            s.count().unwrap(),
            1,
            "Limpiar historial no debe borrar el índice"
        );
    }
}
