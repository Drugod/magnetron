//! eD2k TCP server search only. No Kad, file transfer or embedded eMule.
use crate::{
    dht,
    model::Torrent,
    sources::{Batch, TorrentSource},
};
use futures_util::{future::BoxFuture, stream, StreamExt};
use serde::Deserialize;
use std::{
    io::Read,
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
const MAX_PACKET: usize = 2 * 1024 * 1024;

#[derive(Clone, Deserialize)]
struct Server {
    ip: Ipv4Addr,
    port: u16,
    #[serde(default)]
    name: String,
    #[serde(default)]
    files: u64,
}
#[derive(Deserialize)]
struct List {
    servers: Vec<Server>,
}
fn fallback() -> Vec<Server> {
    [
        ("176.125.242.230", 5555, "eNode"),
        ("176.123.5.89", 4725, "eMule Sunrise"),
        ("85.17.116.222", 6082, "ed2k-rust"),
    ]
    .into_iter()
    .map(|(ip, port, name)| Server {
        ip: ip.parse().unwrap(),
        port,
        name: name.into(),
        files: 0,
    })
    .collect()
}
async fn servers() -> Vec<Server> {
    let load = async {
        let c = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(8))
            .build()
            .ok()?;
        let mut r = c
            .get("https://telemacore.github.io/ed2k-server-list/servers.json")
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = r.chunk().await.ok()? {
            if bytes.len() + chunk.len() > 512 * 1024 {
                return None;
            }
            bytes.extend(chunk);
        }
        let mut list: List = serde_json::from_slice(&bytes).ok()?;
        list.servers
            .retain(|s| dht::public(SocketAddrV4::new(s.ip, s.port)));
        list.servers.sort_by_key(|s| std::cmp::Reverse(s.files));
        let mut seen = std::collections::HashSet::new();
        list.servers.retain(|s| seen.insert((s.ip, s.port)));
        list.servers.truncate(32);
        if list.servers.is_empty() {
            None
        } else {
            Some(list.servers)
        }
    };
    load.await.unwrap_or_else(fallback)
}
fn encoded_name(name: &str) -> String {
    let mut out = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char)
        } else {
            out.push_str(&format!("%{b:02X}"))
        }
    }
    out
}
pub fn link(name: &str, size: u64, hash: &str) -> String {
    format!(
        "ed2k://|file|{}|{size}|{}|/",
        encoded_name(name),
        hash.to_lowercase()
    )
}
pub fn validate_link(uri: &str) -> Result<String, String> {
    if uri.len() > 16384 || uri.chars().any(char::is_control) {
        return Err("Enlace eD2k inválido".into());
    }
    let p: Vec<_> = uri.split('|').collect();
    if p.len() != 6
        || p[0] != "ed2k://"
        || p[1] != "file"
        || p[5] != "/"
        || p[2].is_empty()
        || p[4].len() != 32
        || !p[4].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Solo se permiten enlaces eD2k de fichero".into());
    }
    let size: u64 = p[3].parse().map_err(|_| "Tamaño eD2k inválido")?;
    if size == 0 {
        return Err("Tamaño vacío".into());
    }
    let bytes = p[2].as_bytes();
    let mut decoded = vec![];
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let pair = bytes.get(i + 1..i + 3).ok_or("Escape inválido")?;
            let txt = std::str::from_utf8(pair).map_err(|_| "Escape inválido")?;
            decoded.push(u8::from_str_radix(txt, 16).map_err(|_| "Escape inválido")?);
            i += 3
        } else {
            decoded.push(bytes[i]);
            i += 1
        }
    }
    let name = String::from_utf8(decoded).map_err(|_| "Nombre eD2k inválido")?;
    if name.len() > 4096 || name.chars().any(|c| c.is_control() || c == '|') {
        return Err("Nombre eD2k inválido".into());
    }
    Ok(link(&name, size, p[4]))
}
async fn send(s: &mut TcpStream, op: u8, body: &[u8]) -> Result<(), String> {
    s.write_u8(0xe3).await.map_err(|e| e.to_string())?;
    s.write_u32_le((body.len() + 1) as u32)
        .await
        .map_err(|e| e.to_string())?;
    s.write_u8(op).await.map_err(|e| e.to_string())?;
    s.write_all(body).await.map_err(|e| e.to_string())
}
async fn receive(s: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
    let protocol = s.read_u8().await.map_err(|e| e.to_string())?;
    let len = s.read_u32_le().await.map_err(|e| e.to_string())? as usize;
    if ![0xe3, 0xd4].contains(&protocol) || len == 0 || len > MAX_PACKET {
        return Err("Paquete eD2k inválido o excesivo".into());
    }
    let opcode = s.read_u8().await.map_err(|e| e.to_string())?;
    let mut body = vec![0; len - 1];
    s.read_exact(&mut body).await.map_err(|e| e.to_string())?;
    if protocol == 0xd4 {
        let mut decoded = vec![];
        flate2::read::ZlibDecoder::new(body.as_slice())
            .take((MAX_PACKET + 1) as u64)
            .read_to_end(&mut decoded)
            .map_err(|_| "Compresión eD2k inválida")?;
        if decoded.len() > MAX_PACKET {
            return Err("Paquete eD2k descomprimido excesivo".into());
        }
        body = decoded;
    }
    Ok((opcode, body))
}
fn tag_int(out: &mut Vec<u8>, id: u8, value: u32) {
    out.extend([3, 1, 0, id]);
    out.extend(value.to_le_bytes())
}
fn login() -> Vec<u8> {
    let mut hash: [u8; 16] = rand::random();
    hash[5] = 14;
    hash[14] = 111;
    let mut b = hash.to_vec();
    b.extend(0u32.to_le_bytes());
    b.extend(0u16.to_le_bytes());
    b.extend(3u32.to_le_bytes());
    b.extend([2, 1, 0, 1, 6, 0]);
    b.extend(b"MAGNETRON");
    tag_int(&mut b, 0x11, 60);
    tag_int(&mut b, 0x20, 0x119);
    b
}
fn search_request(query: &str) -> Vec<u8> {
    let words: Vec<_> = query.split_whitespace().take(32).collect();
    fn write(words: &[&str], out: &mut Vec<u8>) {
        if words.len() > 1 {
            out.extend([0, 0]);
            write(&words[..1], out);
            write(&words[1..], out)
        } else if let Some(word) = words.first() {
            out.push(1);
            out.extend((word.len() as u16).to_le_bytes());
            out.extend(word.as_bytes())
        }
    }
    let mut out = vec![];
    write(&words, &mut out);
    out
}
struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.p.checked_add(n).ok_or("Longitud inválida")?;
        let b = self.b.get(self.p..end).ok_or("Paquete eD2k truncado")?;
        self.p = end;
        Ok(b)
    }
    fn n(&mut self, n: usize) -> Result<u64, String> {
        let b = self.take(n)?;
        let mut v = [0; 8];
        v[..n].copy_from_slice(b);
        Ok(u64::from_le_bytes(v))
    }
    fn text(&mut self, n: usize) -> Result<String, String> {
        let b = self.take(n)?;
        Ok(String::from_utf8_lossy(b)
            .trim_start_matches('\u{feff}')
            .to_string())
    }
    fn tag(&mut self) -> Result<(u8, Option<String>, Option<u64>), String> {
        let kind = self.n(1)? as u8;
        let id = if kind & 0x80 != 0 {
            self.n(1)? as u8
        } else {
            let n = self.n(2)? as usize;
            if n == 1 {
                self.n(1)? as u8
            } else {
                self.take(n)?;
                0
            }
        };
        let mut text = None;
        let mut num = None;
        match kind & 0x7f {
            1 => {
                self.take(16)?;
            }
            2 => {
                let n = self.n(2)? as usize;
                text = Some(self.text(n)?);
            }
            3 => num = Some(self.n(4)?),
            4 => {
                self.take(4)?;
            }
            5 => {
                self.take(1)?;
            }
            6 => {
                let n = self.n(2)? as usize;
                self.take(n.div_ceil(8))?;
            }
            7 => {
                let n = self.n(4)? as usize;
                self.take(n)?;
            }
            8 => num = Some(self.n(2)?),
            9 => num = Some(self.n(1)?),
            10 => {
                let n = self.n(1)? as usize;
                self.take(n)?;
            }
            11 => num = Some(self.n(8)?),
            k @ 0x11..=0x20 => text = Some(self.text((k - 0x10) as usize)?),
            _ => return Err("Tipo de etiqueta eD2k desconocido".into()),
        }
        Ok((id, text, num))
    }
}
fn parse_results(bytes: &[u8], source: &str) -> Result<(Vec<Torrent>, bool), String> {
    let mut r = Reader { b: bytes, p: 0 };
    let count = r.n(4)? as usize;
    if count > 1000 {
        return Err("Demasiados resultados eD2k".into());
    }
    let mut rows = vec![];
    for _ in 0..count {
        let hash = hex::encode(r.take(16)?);
        r.take(6)?;
        let tags = r.n(4)?;
        if tags > 256 {
            return Err("Demasiadas etiquetas eD2k".into());
        }
        let (mut name, mut size, mut hi, mut complete, mut kind) =
            (String::new(), 0u64, 0u64, None, String::new());
        for _ in 0..tags {
            let (id, text, num) = r.tag()?;
            match id {
                1 => name = text.unwrap_or_default(),
                2 => size = num.unwrap_or(0),
                3 => kind = text.unwrap_or_default(),
                0x3a => hi = num.unwrap_or(0),
                0x30 => complete = num,
                _ => {}
            }
        }
        if hi > u32::MAX as u64 {
            return Err("Tamaño eD2k inválido".into());
        }
        size |= hi << 32;
        if name.is_empty()
            || name.len() > 4096
            || name.chars().any(|c| c.is_control() || c == '|')
            || size == 0
        {
            continue;
        }
        let content = match kind.to_lowercase().as_str() {
            "video" => Some("video"),
            "audio" => Some("audio"),
            "doc" => Some("documents"),
            "arc" => Some("archives"),
            "pro" => Some("software"),
            "image" => Some("images"),
            "cdimage" => Some("iso"),
            _ => None,
        };
        rows.push(Torrent {
            id: format!("ed2k:{hash}:{size}"),
            title: name.clone(),
            infohash_v1: String::new(),
            magnet_uri: String::new(),
            size_bytes: Some(size),
            published_at: None,
            seeds_reported: None,
            availability_status: if complete.unwrap_or(0) > 0 {
                "reported_complete"
            } else {
                "unknown"
            }
            .into(),
            availability_checked_at: Some(chrono::Utc::now().to_rfc3339()),
            sources: vec![source.into()],
            files: vec![name.clone()],
            content_types: content.map(|t| vec![t.into()]).unwrap_or_default(),
            network: "ed2k".into(),
            ed2k_uri: Some(link(&name, size, &hash)),
            complete_sources_reported: complete,
            discovered_at: chrono::Utc::now().to_rfc3339(),
        });
    }
    Ok((rows, bytes.get(r.p) == Some(&1)))
}
async fn search_server(server: &Server, query: &str, batch: Batch) -> Result<usize, String> {
    let addr = SocketAddrV4::new(server.ip, server.port);
    if !dht::public(addr) {
        return Err("Servidor no público".into());
    }
    let mut s = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
    send(&mut s, 1, &login()).await?;
    let mut logged = false;
    for _ in 0..32 {
        let (op, b) = receive(&mut s).await?;
        if op == 0x40 && b.len() >= 4 {
            logged = true;
            break;
        }
        if op == 5 {
            return Err("Servidor eD2k rechazó la conexión".into());
        }
    }
    if !logged {
        return Err("Servidor eD2k sin respuesta de acceso".into());
    }
    send(&mut s, 0x16, &search_request(query)).await?;
    let source = format!("ed2k:{}:{}", server.ip, server.port);
    let mut total = 0;
    let mut pages = 0;
    for _ in 0..64 {
        let (op, b) = receive(&mut s).await?;
        if op == 0x33 {
            let (rows, more) = parse_results(&b, &source)?;
            total += rows.len();
            batch(rows);
            pages += 1;
            if !more || pages >= 3 {
                return Ok(total);
            }
            send(&mut s, 0x21, &[]).await?;
        }
        if op == 5 {
            return Err("El servidor eD2k rechazó la búsqueda".into());
        }
    }
    Ok(total)
}
pub async fn recheck(item: &Torrent) -> Result<Torrent, String> {
    for source in item
        .sources
        .iter()
        .filter_map(|s| s.strip_prefix("ed2k:"))
        .take(2)
    {
        let Ok(addr) = source.parse::<SocketAddrV4>() else {
            continue;
        };
        if !dht::public(addr) {
            continue;
        }
        let server = Server {
            ip: *addr.ip(),
            port: addr.port(),
            name: String::new(),
            files: 0,
        };
        let found = std::sync::Arc::new(std::sync::Mutex::new(None));
        let output = found.clone();
        let id = item.id.clone();
        let batch: Batch = std::sync::Arc::new(move |rows| {
            for row in rows {
                if row.id == id && row.complete_sources_reported.unwrap_or(0) > 0 {
                    *output.lock().unwrap() = Some(row);
                }
            }
        });
        let _ = tokio::time::timeout(
            Duration::from_secs(12),
            search_server(&server, &item.title, batch),
        )
        .await;
        let matched = found.lock().unwrap().take();
        if let Some(row) = matched {
            return Ok(row);
        }
    }
    Err("El servidor ya no confirma fuentes completas. Repite la búsqueda más tarde.".into())
}
pub struct Ed2k;
impl TorrentSource for Ed2k {
    fn id(&self) -> &str {
        "ed2k"
    }
    fn search<'a>(
        &'a self,
        _client: &'a reqwest::Client,
        query: &'a str,
        batch: Batch,
    ) -> BoxFuture<'a, Result<usize, String>> {
        Box::pin(async move {
            let jobs = servers().await.into_iter().map(|server| {
                let batch = batch.clone();
                async move {
                    tokio::time::timeout(
                        Duration::from_secs(18),
                        search_server(&server, query, batch),
                    )
                    .await
                }
            });
            let mut jobs = stream::iter(jobs).buffer_unordered(2);
            let mut total = 0;
            let mut responded = 0;
            while let Some(r) = jobs.next().await {
                if let Ok(Ok(n)) = r {
                    responded += 1;
                    total += n
                }
            }
            if responded == 0 {
                Err("No respondió ningún servidor eD2k".into())
            } else {
                Ok(total)
            }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_file_links_are_accepted() {
        let uri = link("Ubuntu 26.10.iso", 12345, &"ab".repeat(16));
        assert_eq!(validate_link(&uri).unwrap(), uri);
        for bad in [
            "ed2k://|server|1.2.3.4|4661|/",
            "file:///c:/foo",
            "ed2k://|file|x|0|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|/",
            "ed2k://|file|x%7Cy|42|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|/",
            "ed2k://|file|x%00|42|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|/",
        ] {
            assert!(validate_link(bad).is_err());
        }
    }
    #[test]
    fn malformed_packets_are_rejected() {
        assert!(parse_results(&[255, 255, 255, 255], "ed2k:test").is_err());
        assert!(parse_results(&[1, 0, 0, 0], "ed2k:test").is_err());
        assert_eq!(
            search_request("ubuntu iso"),
            [
                vec![0, 0, 1, 6, 0],
                b"ubuntu".to_vec(),
                vec![1, 3, 0],
                b"iso".to_vec()
            ]
            .concat()
        );
    }
    fn fixture() -> Vec<u8> {
        let mut b = 1u32.to_le_bytes().to_vec();
        b.extend([0xab; 16]);
        b.extend([0; 6]);
        b.extend(4u32.to_le_bytes());
        b.extend([2, 1, 0, 1, 10, 0]);
        b.extend(b"ubuntu.iso");
        tag_int(&mut b, 2, 12345);
        tag_int(&mut b, 0x30, 2);
        tag_int(&mut b, 0x3a, 1);
        b.push(0);
        b
    }
    #[test]
    fn server_results_preserve_identity_large_size_and_reported_evidence() {
        let b = fixture();
        let (rows, more) = parse_results(&b, "ed2k:8.8.8.8:4661").unwrap();
        let r = &rows[0];
        assert!(!more);
        assert_eq!(r.size_bytes, Some((1u64 << 32) + 12345));
        assert_eq!(r.complete_sources_reported, Some(2));
        assert_eq!(r.availability_status, "reported_complete");
        assert_eq!(r.network, "ed2k");
        assert_eq!(r.infohash_v1, "");
        assert!(r.id.starts_with("ed2k:"));
        assert!(validate_link(r.ed2k_uri.as_ref().unwrap()).is_ok());
        for n in 0..b.len() - 1 {
            assert!(parse_results(&b[..n], "ed2k:test").is_err());
        }
    }
    #[tokio::test]
    async fn compressed_packets_are_bounded_and_read_correctly() {
        use std::io::Write;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let raw = fixture();
            let mut encoder =
                flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
            encoder.write_all(&raw).unwrap();
            let zipped = encoder.finish().unwrap();
            s.write_u8(0xd4).await.unwrap();
            s.write_u32_le((zipped.len() + 1) as u32).await.unwrap();
            s.write_u8(0x33).await.unwrap();
            s.write_all(&zipped).await.unwrap();
            s.write_u8(0xe3).await.unwrap();
            s.write_u32_le((MAX_PACKET + 1) as u32).await.unwrap();
        });
        let mut s = TcpStream::connect(addr).await.unwrap();
        let (op, b) = receive(&mut s).await.unwrap();
        assert_eq!(op, 0x33);
        assert_eq!(parse_results(&b, "ed2k:test").unwrap().0.len(), 1);
        assert!(receive(&mut s).await.is_err());
        peer.await.unwrap();
    }
    #[tokio::test]
    #[ignore = "Consulta servidores eD2k públicos con Ubuntu"]
    async fn live_ed2k() {
        let rows = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        for server in servers().await.into_iter().take(5) {
            let output = rows.clone();
            let result = tokio::time::timeout(
                Duration::from_secs(20),
                search_server(
                    &server,
                    "ubuntu",
                    std::sync::Arc::new(move |b| output.lock().unwrap().extend(b)),
                ),
            )
            .await;
            println!(
                "{} {}:{} -> {result:?}",
                server.name, server.ip, server.port
            );
        }
        let rows = rows.lock().unwrap();
        let complete = rows
            .iter()
            .filter(|r| r.complete_sources_reported.unwrap_or(0) > 0)
            .count();
        println!(
            "{} resultados, {} con fuentes completas reportadas",
            rows.len(),
            complete
        );
        for r in rows.iter().take(3) {
            println!(
                "{} {:?} {:?}",
                r.title, r.complete_sources_reported, r.ed2k_uri
            );
        }
        assert!(
            complete > 0,
            "No llegaron resultados con fuentes completas desde los servidores probados"
        );
        let selected = rows
            .iter()
            .find(|r| {
                r.complete_sources_reported.unwrap_or(0) > 0
                    && r.title.to_lowercase().ends_with(".iso")
            })
            .or_else(|| {
                rows.iter()
                    .find(|r| r.complete_sources_reported.unwrap_or(0) > 0)
            })
            .unwrap()
            .clone();
        drop(rows);
        let fresh = recheck(&selected).await;
        println!(
            "Nueva consulta antes de abrir: {:?}",
            fresh.as_ref().map(|r| (&r.id, r.complete_sources_reported))
        );
        assert!(
            fresh.is_ok(),
            "No se pudo renovar la evidencia del servidor"
        );
    }
}
