use crate::model::Torrent;
use sha1::{Digest, Sha1};

// Preserve the original bencoded info bytes: re-encoding would change the infohash.
fn skip(data: &[u8], pos: &mut usize, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err("Metadatos demasiado anidados".into());
    }
    match data.get(*pos) {
        Some(b'i') => {
            *pos += 1;
            let start = *pos;
            while data.get(*pos).is_some_and(|b| *b != b'e') {
                *pos += 1;
            }
            if *pos == start || data.get(*pos) != Some(&b'e') {
                return Err("Entero inválido".into());
            }
            std::str::from_utf8(&data[start..*pos])
                .map_err(|_| "Entero inválido")?
                .parse::<i64>()
                .map_err(|_| "Entero inválido")?;
            *pos += 1;
        }
        Some(b'l' | b'd') => {
            *pos += 1;
            while data.get(*pos).is_some_and(|b| *b != b'e') {
                skip(data, pos, depth + 1)?;
            }
            if data.get(*pos) != Some(&b'e') {
                return Err("Diccionario truncado".into());
            }
            *pos += 1;
        }
        Some(b'0'..=b'9') => {
            bytes(data, pos)?;
        }
        _ => return Err("Bencode inválido".into()),
    }
    Ok(())
}
pub(crate) fn bencode_end(data: &[u8]) -> Result<usize, String> {
    let mut pos = 0;
    skip(data, &mut pos, 0)?;
    Ok(pos)
}
fn bytes<'a>(data: &'a [u8], pos: &mut usize) -> Result<&'a [u8], String> {
    let start = *pos;
    while data.get(*pos).is_some_and(|b| b.is_ascii_digit()) {
        *pos += 1;
    }
    if *pos == start || data.get(*pos) != Some(&b':') {
        return Err("Cadena inválida".into());
    }
    let len = std::str::from_utf8(&data[start..*pos])
        .map_err(|_| "Longitud inválida")?
        .parse::<usize>()
        .map_err(|_| "Longitud inválida")?;
    *pos += 1;
    let end = pos.checked_add(len).ok_or("Longitud inválida")?;
    let value = data.get(*pos..end).ok_or("Cadena truncada")?;
    *pos = end;
    Ok(value)
}
pub(crate) fn field<'a>(data: &'a [u8], name: &[u8]) -> Result<Option<&'a [u8]>, String> {
    if data.first() != Some(&b'd') {
        return Err("Se esperaba un diccionario".into());
    }
    let mut pos = 1;
    while data.get(pos).is_some_and(|b| *b != b'e') {
        let key = bytes(data, &mut pos)?;
        let start = pos;
        skip(data, &mut pos, 0)?;
        if key == name {
            return Ok(Some(&data[start..pos]));
        }
    }
    Ok(None)
}
fn integer(data: &[u8]) -> Option<u64> {
    std::str::from_utf8(data)
        .ok()?
        .strip_prefix('i')?
        .strip_suffix('e')?
        .parse()
        .ok()
}
// Count pieces only from hash-verified metadata, checking the content geometry.
pub(crate) fn piece_count(info: &[u8]) -> Result<usize, String> {
    if field(info, b"private")?.and_then(integer) == Some(1) {
        return Err("Torrent privado: no se comprueba mediante DHT".into());
    }
    let encoded = field(info, b"pieces")?.ok_or("Sin hashes de piezas")?;
    let mut pos = 0;
    let hashes = bytes(encoded, &mut pos)?;
    if hashes.is_empty() || hashes.len() % 20 != 0 {
        return Err("Lista de piezas inválida".into());
    }
    let length = field(info, b"piece length")?
        .and_then(integer)
        .filter(|n| *n > 0)
        .ok_or("Longitud de pieza inválida")?;
    let mut total = field(info, b"length")?.and_then(integer);
    if total.is_none() {
        let files = field(info, b"files")?.ok_or("Sin tamaño de contenido")?;
        if files.first() != Some(&b'l') {
            return Err("Lista de ficheros inválida".into());
        }
        let mut pos = 1;
        let mut sum = 0u64;
        while files.get(pos).is_some_and(|b| *b != b'e') {
            let start = pos;
            skip(files, &mut pos, 0)?;
            let n = field(&files[start..pos], b"length")?
                .and_then(integer)
                .ok_or("Tamaño inválido")?;
            sum = sum.checked_add(n).ok_or("Tamaño excesivo")?;
        }
        total = Some(sum);
    }
    let count = hashes.len() / 20;
    if total.unwrap().div_ceil(length) != count as u64 {
        return Err("Piezas y tamaño inconsistentes".into());
    }
    Ok(count)
}
pub fn parse(data: &[u8], source: &str) -> Result<Torrent, String> {
    let mut end = 0;
    skip(data, &mut end, 0)?;
    if end != data.len() {
        return Err("Datos adicionales tras el torrent".into());
    }
    let info = field(data, b"info")?.ok_or("Torrent sin info")?;
    // This release handles v1/hybrid metadata. Pure v2 must not be mislabeled v1.
    if field(info, b"pieces")?.is_none() {
        return Err("Torrent v2 puro no compatible con este conector".into());
    }
    let raw_name = field(info, b"name.utf-8")?
        .or(field(info, b"name")?)
        .ok_or("Torrent sin nombre")?;
    let mut p = 0;
    let title = String::from_utf8_lossy(bytes(raw_name, &mut p)?).to_string();
    let size = if let Some(length) = field(info, b"length")? {
        integer(length)
    } else if let Some(files) = field(info, b"files")? {
        let mut pos = 1;
        let mut total = Some(0u64);
        while files.get(pos).is_some_and(|b| *b != b'e') {
            let start = pos;
            skip(files, &mut pos, 0)?;
            let len = field(&files[start..pos], b"length")?.and_then(integer);
            total = total.zip(len).and_then(|(a, b)| a.checked_add(b));
        }
        total
    } else {
        None
    };
    let hash = hex::encode(Sha1::digest(info));
    let mut file_names = vec![];
    if let Ok(serde_bencode::value::Value::Dict(dictionary)) =
        serde_bencode::from_bytes::<serde_bencode::value::Value>(info)
    {
        if let Some(serde_bencode::value::Value::List(files)) = dictionary.get(b"files".as_slice())
        {
            for file in files.iter().take(2000) {
                if let serde_bencode::value::Value::Dict(file) = file {
                    if let Some(serde_bencode::value::Value::List(path)) = file
                        .get(b"path.utf-8".as_slice())
                        .or_else(|| file.get(b"path".as_slice()))
                    {
                        let parts: Vec<_> = path
                            .iter()
                            .filter_map(|p| {
                                if let serde_bencode::value::Value::Bytes(p) = p {
                                    Some(String::from_utf8_lossy(p).to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        let path = parts.join("/");
                        if !path.is_empty() {
                            file_names.push(path.chars().take(4096).collect());
                        }
                    }
                }
            }
        } else {
            file_names.push(title.clone());
        }
    }
    let mut magnet = url::Url::parse("magnet:").unwrap();
    magnet
        .query_pairs_mut()
        .append_pair("xt", &format!("urn:btih:{hash}"))
        .append_pair("dn", &title);
    if let Some(raw) = field(data, b"announce")? {
        let mut pos = 0;
        let tracker = String::from_utf8_lossy(bytes(raw, &mut pos)?);
        if let Ok(uri) = url::Url::parse(&tracker) {
            if ["https", "http", "udp"].contains(&uri.scheme()) {
                magnet.query_pairs_mut().append_pair("tr", &tracker);
            }
        }
    }
    let published_at = field(data, b"creation date")?
        .and_then(integer)
        .and_then(|n| i64::try_from(n).ok())
        .and_then(|n| chrono::DateTime::from_timestamp(n, 0))
        .map(|d| d.to_rfc3339());
    Ok(Torrent {
        id: hash.clone(),
        title,
        infohash_v1: hash,
        magnet_uri: magnet.into(),
        size_bytes: size,
        published_at,
        seeds_reported: None,
        availability_status: "unknown".into(),
        availability_checked_at: None,
        sources: vec![source.into()],
        files: file_names,
        content_types: vec![],
        network: "bittorrent".into(),
        ed2k_uri: None,
        complete_sources_reported: None,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_original_info() {
        let info = b"d6:lengthi42e4:name10:ubuntu.iso6:pieces0:e";
        let mut raw = b"d4:info".to_vec();
        raw.extend(info);
        raw.push(b'e');
        let t = parse(&raw, "test").unwrap();
        assert_eq!(t.size_bytes, Some(42));
        assert_eq!(t.title, "ubuntu.iso");
        assert_eq!(t.infohash_v1, hex::encode(Sha1::digest(info)));
        assert!(crate::model::validate_magnet(&t.magnet_uri).is_ok());
    }
    #[test]
    fn bad_metadata() {
        for raw in [
            b"".as_slice(),
            b"d4:info999:x",
            b"d4:infodee",
            b"d4:infod4:name1:xee",
        ] {
            assert!(parse(raw, "test").is_err());
        }
    }
}
