//! Bounded local torrent imports. Keep metadata in private app state, never depend on the dropped path.
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

pub const MAX_BYTES: usize = 4 * 1024 * 1024;
pub struct Imported {
    pub bytes: Vec<u8>,
    pub uri: String,
    pub name: String,
    pub pure_v2: bool,
}
pub fn read(path: &Path) -> Result<Imported, String> {
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("torrent"))
    {
        return Err("Arrastra un archivo .torrent válido".into());
    }
    let file = std::fs::File::open(path).map_err(|_| "No se pudo leer el archivo .torrent")?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Arrastra un archivo .torrent válido".into());
    }
    let mut data = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    parse(data)
}
pub fn parse(data: Vec<u8>) -> Result<Imported, String> {
    if data.is_empty() || data.len() > MAX_BYTES {
        return Err("El archivo .torrent está vacío o supera 4 MiB".into());
    }
    if crate::torrent::bencode_end(&data)? != data.len() {
        return Err("Archivo .torrent inválido".into());
    }
    let info = crate::torrent::field(&data, b"info")?.ok_or("Archivo .torrent sin metadatos")?;
    let pure_v2 = crate::torrent::field(info, b"pieces")?.is_none();
    let mut uri = url::Url::parse("magnet:").unwrap();
    let name;
    if pure_v2 {
        if crate::torrent::field(info, b"meta version")? != Some(b"i2e".as_slice()) {
            return Err("Archivo .torrent inválido".into());
        }
        let raw = crate::torrent::field(info, b"name")?.ok_or("Torrent sin nombre")?;
        let serde_bencode::value::Value::Bytes(raw) =
            serde_bencode::from_bytes(raw).map_err(|_| "Torrent sin nombre")?
        else {
            return Err("Torrent sin nombre".into());
        };
        name = String::from_utf8_lossy(&raw)
            .chars()
            .take(500)
            .collect::<String>();
        uri.query_pairs_mut().append_pair(
            "xt",
            &format!("urn:btmh:1220{}", hex::encode(Sha256::digest(info))),
        );
    } else {
        let metadata =
            librqbit::torrent_from_bytes(&data).map_err(|_| "Archivo .torrent inválido")?;
        let details = metadata
            .info
            .data
            .validate()
            .map_err(|_| "Archivo .torrent inválido")?;
        for file in details.iter_file_details() {
            crate::downloads::validate_file_path(&file.filename.to_pathbuf())?;
        }
        // Parse display text only; the complete original metadata (including trackers) is retained.
        name = crate::torrent::parse(&data, "local")?
            .title
            .chars()
            .take(500)
            .collect();
        uri.query_pairs_mut().append_pair(
            "xt",
            &format!("urn:btih:{}", metadata.info_hash.as_string()),
        );
    }
    uri.query_pairs_mut().append_pair("dn", &name);
    Ok(Imported {
        bytes: data,
        uri: uri.into(),
        name,
        pure_v2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn v1() -> Vec<u8> {
        let mut data =
            b"d4:infod6:lengthi1e4:name5:a.txt12:piece lengthi16384e6:pieces20:".to_vec();
        data.extend([0; 20]);
        data.extend(b"ee");
        data
    }
    #[test]
    fn validates_local_metadata_and_identity() {
        let imported = parse(v1()).unwrap();
        assert!(!imported.pure_v2);
        assert_eq!(imported.name, "a.txt");
        let uri = url::Url::parse(&imported.uri).unwrap();
        let expected = librqbit::torrent_from_bytes(&v1())
            .unwrap()
            .info_hash
            .as_string();
        assert!(uri
            .query_pairs()
            .any(|(key, value)| key == "xt" && value == format!("urn:btih:{expected}")));
        let mut trailing = v1();
        trailing.push(b'e');
        assert!(parse(trailing).is_err());
        assert!(parse(vec![]).is_err());
        assert!(parse(vec![0; MAX_BYTES + 1]).is_err());
        assert!(parse(b"d4:infodee".to_vec()).is_err());
        assert!(parse(b"d4:infod4:name5:a.txtee".to_vec()).is_err());
    }
    #[test]
    fn preserves_hybrid_and_v2_metadata_without_changing_the_info_hash() {
        let hybrid = include_bytes!("../../tests/fixtures/import-hybrid.torrent");
        let pure = include_bytes!("../../tests/fixtures/import-v2.torrent");
        let imported = parse(hybrid.to_vec()).unwrap();
        assert!(!imported.pure_v2);
        assert_eq!(imported.bytes, hybrid);
        let imported = parse(pure.to_vec()).unwrap();
        assert!(imported.pure_v2);
        assert_eq!(imported.bytes, pure);
        assert_eq!(imported.name, "sample.bin");
        let info = crate::torrent::field(pure, b"info").unwrap().unwrap();
        let expected = hex::encode(Sha256::digest(info));
        assert_eq!(
            crate::v2::identity(&imported.uri).unwrap().0,
            format!("bt2:{expected}")
        );
        let malicious = v1().windows(5).position(|w| w == b"a.txt").unwrap();
        let mut bad = v1();
        bad[malicious..malicious + 5].copy_from_slice(b"../ab");
        assert!(parse(bad).is_err());
    }
    #[test]
    fn rejects_non_torrent_and_oversized_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, v1()).unwrap();
        assert!(read(&path).is_err());
        let path = temp.path().join("file.TORRENT");
        std::fs::write(&path, v1()).unwrap();
        assert!(read(&path).is_ok());
        std::fs::write(&path, vec![0; MAX_BYTES + 1]).unwrap();
        assert!(read(&path).is_err());
        assert!(read(Path::new("relative.torrent")).is_err());
    }
}
