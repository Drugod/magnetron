//! Opt-in discovery and BEP 9 metadata only. Never requests content pieces.
use crate::{model::Torrent, storage::Storage, torrent};
use futures_util::StreamExt;
use serde::Serialize;
use serde_bencode::value::Value as B;
use sha1::{Digest, Sha1};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    net::{SocketAddr, SocketAddrV4},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
    task::JoinHandle,
};

#[derive(Clone, Default, Serialize)]
pub struct Status {
    pub running: bool,
    pub nodes: u64,
    pub hashes: u64,
    pub indexed: u64,
    pub failures: u64,
    pub message: String,
}
pub struct Discovery {
    status: Arc<Mutex<Status>>,
    task: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}
impl Discovery {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(Status {
                message: "DHT pausada. Se activa manualmente y no descarga contenido.".into(),
                ..Default::default()
            })),
            task: tokio::sync::Mutex::new(None),
        }
    }
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub async fn start(&self, db: Arc<Mutex<Storage>>) -> Result<(), String> {
        let mut task = self.task.lock().await;
        if task.as_ref().is_some_and(|t| !t.is_finished()) {
            return Ok(());
        }
        *self.status.lock().map_err(|e| e.to_string())? = Status {
            running: true,
            message: "Conectando con la red DHT…".into(),
            ..Default::default()
        };
        let status = self.status.clone();
        *task = Some(tokio::spawn(async move {
            if let Err(e) = run(db, status.clone()).await {
                let mut s = status.lock().unwrap_or_else(|e| e.into_inner());
                s.running = false;
                s.message = e;
            }
        }));
        Ok(())
    }
    pub async fn pause(&self) {
        if let Some(task) = self.task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        let mut status = self.status.lock().unwrap_or_else(|e| e.into_inner());
        status.running = false;
        status.message =
            "DHT pausada. El índice se conserva; al buscar se comprobará su disponibilidad.".into();
    }
}
fn dict(items: Vec<(&[u8], B)>) -> B {
    B::Dict(items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect())
}
fn bytes(value: impl AsRef<[u8]>) -> B {
    B::Bytes(value.as_ref().to_vec())
}
fn get<'a>(v: &'a B, key: &[u8]) -> Option<&'a B> {
    if let B::Dict(d) = v {
        d.get(key)
    } else {
        None
    }
}
fn raw(v: &B) -> Option<&[u8]> {
    if let B::Bytes(v) = v {
        Some(v)
    } else {
        None
    }
}
fn int(v: &B) -> Option<i64> {
    if let B::Int(n) = v {
        Some(*n)
    } else {
        None
    }
}
pub(crate) fn public(addr: SocketAddrV4) -> bool {
    let ip = addr.ip();
    let o = ip.octets();
    addr.port() != 0
        && !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_unspecified()
        && !ip.is_broadcast()
        && !ip.is_multicast()
        && o[0] != 0
        && o[0] < 224
        && !(o[0] == 100 && (64..=127).contains(&o[1]))
        && !(o[0] == 198 && (o[1] == 18 || o[1] == 19))
        && !(o[0] == 192 && o[1] == 0)
        && !(o[0] == 198 && o[1] == 51)
        && !(o[0] == 203 && o[1] == 0 && o[2] == 113)
}
async fn sample(
    node: SocketAddrV4,
    id: &[u8],
    target: &[u8],
) -> Result<(Vec<[u8; 20]>, Vec<SocketAddrV4>, u64), String> {
    if !public(node) {
        return Err("Nodo no público".into());
    }
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| e.to_string())?;
    socket.connect(node).await.map_err(|e| e.to_string())?;
    let tx: [u8; 4] = rand::random();
    let request = dict(vec![
        (b"t", bytes(tx)),
        (b"y", bytes(b"q")),
        (b"q", bytes(b"sample_infohashes")),
        (
            b"a",
            dict(vec![(b"id", bytes(id)), (b"target", bytes(target))]),
        ),
    ]);
    socket
        .send(&serde_bencode::to_bytes(&request).map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 8192];
    let n = tokio::time::timeout(Duration::from_secs(3), socket.recv(&mut buffer))
        .await
        .map_err(|_| "Nodo sin respuesta")?
        .map_err(|e| e.to_string())?;
    let response: B =
        serde_bencode::from_bytes(&buffer[..n]).map_err(|_| "Respuesta DHT inválida")?;
    if get(&response, b"t").and_then(raw) != Some(tx.as_slice())
        || get(&response, b"y").and_then(raw) != Some(b"r".as_slice())
    {
        return Err("Transacción DHT no válida".into());
    }
    let r = get(&response, b"r").ok_or("Sin respuesta DHT")?;
    let mut nodes = vec![];
    if let Some(compact) = get(r, b"nodes").and_then(raw) {
        for n in compact.chunks_exact(26).take(32) {
            let a = SocketAddrV4::new(
                [n[20], n[21], n[22], n[23]].into(),
                u16::from_be_bytes([n[24], n[25]]),
            );
            if public(a) {
                nodes.push(a)
            }
        }
    }
    let hashes = get(r, b"samples")
        .and_then(raw)
        .unwrap_or(&[])
        .chunks_exact(20)
        .take(100)
        .map(|h| h.try_into().unwrap())
        .collect();
    let interval = get(r, b"interval")
        .and_then(int)
        .unwrap_or(21600)
        .clamp(1, 21600) as u64;
    Ok((hashes, nodes, interval))
}
async fn run(db: Arc<Mutex<Storage>>, status: Arc<Mutex<Status>>) -> Result<(), String> {
    let dht = mainline::Dht::builder()
        .port(0)
        .request_timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?
        .as_async();
    let mut queue: VecDeque<[u8; 20]> = db
        .lock()
        .map_err(|e| e.to_string())?
        .known_hashes()?
        .into_iter()
        .filter_map(|h| hex::decode(h).ok()?.try_into().ok())
        .collect();
    let mut seen: HashSet<[u8; 20]> = queue.iter().copied().collect();
    let mut nodes = VecDeque::new();
    let mut sampled: HashMap<SocketAddrV4, Instant> = HashMap::new();
    loop {
        let cfg = db.lock().map_err(|e| e.to_string())?.settings()?;
        if nodes.is_empty() {
            let found=tokio::time::timeout(Duration::from_secs(15),dht.find_node(mainline::Id::random())).await.map_err(|_|"No se pudo conectar con DHT. Comprueba que tu red permite UDP y vuelve a activar el índice.")?;
            nodes.extend(found.iter().map(|n| n.address()).filter(|a| public(*a)));
            status.lock().unwrap().message = if nodes.is_empty() {
                "Sin nodos accesibles; reintentando…"
            } else {
                "Descubriendo identificadores y recuperando solo metadatos."
            }
            .into();
        }
        if let Some(node) = nodes.pop_front() {
            sampled.retain(|_, until| *until > Instant::now());
            if !sampled.contains_key(&node) {
                // Conservative: at most one BEP 51 request per loop, two-second pause.
                sampled.insert(node, Instant::now() + Duration::from_secs(21600));
                let id = *dht.info().await.id().as_bytes();
                if let Ok((hashes, more, interval)) =
                    sample(node, &id, mainline::Id::random().as_bytes()).await
                {
                    sampled.insert(node, Instant::now() + Duration::from_secs(interval));
                    let mut s = status.lock().unwrap();
                    s.nodes += 1;
                    for h in hashes {
                        if seen.len() < cfg.index_max_records.min(100000) as usize
                            && queue.len() < 500
                            && seen.insert(h)
                        {
                            s.hashes += 1;
                            queue.push_back(h);
                        }
                    }
                    if nodes.len() < 500 {
                        nodes.extend(more.into_iter().take(500 - nodes.len()));
                    }
                }
                if sampled.len() > 4096 {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
        }
        if let Some(hash) = queue.pop_front() {
            let result = tokio::time::timeout(
                Duration::from_secs(cfg.discovery_timeout_seconds),
                resolve(&dht, hash),
            )
            .await;
            match result {
                Ok(Ok(item)) => {
                    db.lock().map_err(|e| e.to_string())?.cache(&[item])?;
                    status.lock().unwrap().indexed += 1;
                }
                _ => status.lock().unwrap().failures += 1,
            }
        }
        tokio::time::sleep(Duration::from_secs(cfg.discovery_interval_seconds)).await;
    }
}
async fn resolve(dht: &mainline::async_dht::AsyncDht, hash: [u8; 20]) -> Result<Torrent, String> {
    let mut peers = dht.get_peers(mainline::Id::from_bytes(hash).map_err(|e| e.to_string())?);
    let mut tried = 0;
    while let Some(batch) = peers.next().await {
        for peer in batch.into_iter().filter(|a| public(*a)).take(3) {
            tried += 1;
            if let Ok(Ok(item)) =
                tokio::time::timeout(Duration::from_secs(7), metadata(SocketAddr::V4(peer), hash))
                    .await
            {
                return Ok(item);
            }
            if tried >= 3 {
                return Err("No se obtuvieron metadatos de los peers".into());
            }
        }
    }
    Err("Sin peers disponibles".into())
}
async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let n = stream.read_u32().await.map_err(|e| e.to_string())? as usize;
    if n > 65536 {
        return Err("Mensaje de peer demasiado grande".into());
    }
    let mut data = vec![0; n];
    stream
        .read_exact(&mut data)
        .await
        .map_err(|e| e.to_string())?;
    Ok(data)
}
async fn write_extended(stream: &mut TcpStream, id: u8, data: &[u8]) -> Result<(), String> {
    stream
        .write_u32((data.len() + 2) as u32)
        .await
        .map_err(|e| e.to_string())?;
    stream
        .write_all(&[20, id])
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(data).await.map_err(|e| e.to_string())
}
pub async fn metadata(peer: SocketAddr, hash: [u8; 20]) -> Result<Torrent, String> {
    inspect(peer, hash, false).await
}
fn complete_bitfield(bits: &[u8], pieces: usize) -> bool {
    if pieces == 0 || bits.len() != pieces.div_ceil(8) {
        return false;
    }
    let full = pieces / 8;
    if bits[..full].iter().any(|b| *b != 255) {
        return false;
    }
    pieces % 8 == 0 || bits[full] == (255u8 << (8 - pieces % 8))
}
fn observe(frame: &[u8], bitfield: &mut Option<Vec<u8>>, unchoked: &mut bool) {
    match frame {
        [5, rest @ ..] => *bitfield = Some(rest.to_vec()),
        [1] => *unchoked = true,
        [0] => *unchoked = false,
        _ => {}
    }
}
pub(crate) async fn inspect(
    peer: SocketAddr,
    hash: [u8; 20],
    require_complete: bool,
) -> Result<Torrent, String> {
    let mut bitfield = None;
    let mut unchoked = false;
    let mut stream = TcpStream::connect(peer).await.map_err(|e| e.to_string())?;
    let mut handshake = vec![19];
    handshake.extend(b"BitTorrent protocol");
    let mut reserved = [0u8; 8];
    reserved[5] = 0x10;
    handshake.extend(reserved);
    handshake.extend(hash);
    let peer_id: [u8; 20] = rand::random();
    handshake.extend(peer_id);
    stream
        .write_all(&handshake)
        .await
        .map_err(|e| e.to_string())?;
    let mut remote = [0u8; 68];
    stream
        .read_exact(&mut remote)
        .await
        .map_err(|e| e.to_string())?;
    if remote[0] != 19
        || &remote[1..20] != b"BitTorrent protocol"
        || remote[25] & 0x10 == 0
        || remote[28..48] != hash
    {
        return Err("Handshake BitTorrent no válido".into());
    }
    write_extended(&mut stream, 0, b"d1:md11:ut_metadatai1eee").await?;
    let mut ext = None;
    for _ in 0..30 {
        let frame = read_frame(&mut stream).await?;
        observe(&frame, &mut bitfield, &mut unchoked);
        if frame.starts_with(&[20, 0]) {
            let v: B = serde_bencode::from_bytes(&frame[2..])
                .map_err(|_| "Handshake de extensión inválido")?;
            let id = get(&v, b"m")
                .and_then(|m| get(m, b"ut_metadata"))
                .and_then(int)
                .filter(|n| *n > 0 && *n < 256)
                .ok_or("Peer sin extensión de metadatos")? as u8;
            let size = get(&v, b"metadata_size")
                .and_then(int)
                .filter(|n| *n > 0 && *n <= 4 * 1024 * 1024)
                .ok_or("Tamaño de metadatos no permitido")? as usize;
            ext = Some((id, size));
            break;
        }
    }
    let (id, size) = ext.ok_or("No llegó el handshake de metadatos")?;
    let mut info = Vec::with_capacity(size);
    for piece in 0..size.div_ceil(16384) {
        let request = serde_bencode::to_bytes(&dict(vec![
            (b"msg_type", B::Int(0)),
            (b"piece", B::Int(piece as i64)),
        ]))
        .map_err(|e| e.to_string())?;
        write_extended(&mut stream, id, &request).await?;
        let mut received = false;
        for _ in 0..30 {
            let frame = read_frame(&mut stream).await?;
            observe(&frame, &mut bitfield, &mut unchoked);
            if !frame.starts_with(&[20, 1]) {
                continue;
            }
            let end = torrent::bencode_end(&frame[2..])?;
            let header: B = serde_bencode::from_bytes(&frame[2..2 + end])
                .map_err(|_| "Cabecera de metadatos inválida")?;
            if get(&header, b"piece").and_then(int) != Some(piece as i64) {
                continue;
            }
            if get(&header, b"msg_type").and_then(int) != Some(1)
                || get(&header, b"total_size").and_then(int) != Some(size as i64)
            {
                return Err("Metadatos rechazados o inconsistentes".into());
            }
            let data = &frame[2 + end..];
            if data.len() != 16384.min(size - piece * 16384) {
                return Err("Bloque de metadatos truncado".into());
            }
            info.extend(data);
            received = true;
            break;
        }
        if !received {
            return Err("Peer sin respuesta de metadatos".into());
        }
    }
    if Sha1::digest(&info).as_slice() != hash {
        return Err("El hash de los metadatos no coincide".into());
    }
    if require_complete {
        let pieces = torrent::piece_count(&info)?;
        // Interested, but never a content request (wire message 6).
        stream
            .write_all(&[0, 0, 0, 1, 2])
            .await
            .map_err(|e| e.to_string())?;
        for _ in 0..64 {
            if bitfield
                .as_deref()
                .is_some_and(|b| complete_bitfield(b, pieces))
                && unchoked
            {
                break;
            }
            let frame = read_frame(&mut stream).await?;
            observe(&frame, &mut bitfield, &mut unchoked);
        }
        if !unchoked
            || !bitfield
                .as_deref()
                .is_some_and(|b| complete_bitfield(b, pieces))
        {
            return Err("No hay evidencia de una fuente completa accesible".into());
        }
    }
    let mut raw = b"d4:info".to_vec();
    raw.extend(info);
    raw.push(b'e');
    let mut item = torrent::parse(&raw, "dht")?;
    if require_complete {
        item.availability_status = "verified".into();
        item.availability_checked_at = Some(chrono::Utc::now().to_rfc3339());
    }
    Ok(item)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_requires_every_piece_and_valid_spare_bits() {
        assert!(complete_bitfield(&[255, 128], 9));
        assert!(complete_bitfield(&[255], 8));
        for (bits, n) in [
            (vec![], 0),
            (vec![254], 8),
            (vec![255, 0], 9),
            (vec![255, 255], 9),
            (vec![255], 9),
        ] {
            assert!(!complete_bitfield(&bits, n));
        }
    }
    async fn availability_peer(
        bits: Option<u8>,
        unchoke: bool,
        corrupt: bool,
    ) -> Result<Torrent, String> {
        let mut info =
            b"d6:lengthi42e4:name10:ubuntu.iso12:piece lengthi16384e6:pieces20:".to_vec();
        info.extend([7; 20]);
        info.push(b'e');
        let hash: [u8; 20] = Sha1::digest(&info).into();
        if corrupt {
            info[12] = b'3';
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut handshake = [0; 68];
            s.read_exact(&mut handshake).await.unwrap();
            s.write_all(&handshake).await.unwrap();
            assert_eq!(&read_frame(&mut s).await.unwrap()[..2], &[20, 0]);
            if let Some(bits) = bits {
                s.write_all(&[0, 0, 0, 2, 5, bits]).await.unwrap();
            }
            write_extended(
                &mut s,
                0,
                format!("d1:md11:ut_metadatai3ee13:metadata_sizei{}ee", info.len()).as_bytes(),
            )
            .await
            .unwrap();
            let request = read_frame(&mut s).await.unwrap();
            assert_eq!(&request[..2], &[20, 3]);
            let mut data =
                format!("d8:msg_typei1e5:piecei0e10:total_sizei{}ee", info.len()).into_bytes();
            data.extend(info);
            write_extended(&mut s, 1, &data).await.unwrap();
            if corrupt {
                return;
            }
            assert_eq!(read_frame(&mut s).await.unwrap(), vec![2]);
            if unchoke {
                s.write_all(&[0, 0, 0, 1, 1]).await.unwrap();
            }
            // A check may ask for metadata and interested, never a payload block.
            let mut extra = [0; 5];
            let n = tokio::time::timeout(Duration::from_millis(300), s.read(&mut extra)).await;
            assert!(matches!(n, Ok(Ok(0)) | Err(_)));
        });
        let result = tokio::time::timeout(Duration::from_secs(1), inspect(addr, hash, true))
            .await
            .map_err(|_| "Sin disponibilidad".to_string())
            .and_then(|r| r);
        peer.await.unwrap();
        result
    }
    #[tokio::test]
    async fn admits_only_a_complete_unchoked_peer_with_matching_metadata() {
        let item = availability_peer(Some(128), true, false).await.unwrap();
        assert_eq!(item.availability_status, "verified");
        assert!(item.availability_checked_at.is_some());
        assert!(availability_peer(Some(0), true, false).await.is_err());
        assert!(availability_peer(None, true, false).await.is_err());
        assert!(availability_peer(Some(128), false, false).await.is_err());
        assert!(availability_peer(Some(128), true, true).await.is_err());
    }
    #[test]
    fn excludes_private_peers() {
        for ip in [
            "127.0.0.1:80",
            "10.0.0.1:80",
            "192.168.1.1:80",
            "169.254.1.1:80",
            "100.64.1.1:80",
        ] {
            assert!(!public(ip.parse().unwrap()));
        }
    }
    #[tokio::test]
    async fn metadata_only_peer() {
        let info = b"d6:lengthi42e4:name10:ubuntu.iso6:pieces0:e".to_vec();
        let hash: [u8; 20] = Sha1::digest(&info).into();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut handshake = [0; 68];
            s.read_exact(&mut handshake).await.unwrap();
            s.write_all(&handshake).await.unwrap();
            assert_eq!(&read_frame(&mut s).await.unwrap()[..2], &[20, 0]);
            write_extended(
                &mut s,
                0,
                format!("d1:md11:ut_metadatai3ee13:metadata_sizei{}ee", info.len()).as_bytes(),
            )
            .await
            .unwrap();
            let req = read_frame(&mut s).await.unwrap();
            assert_eq!(&req[..2], &[20, 3]);
            let mut response =
                format!("d8:msg_typei1e5:piecei0e10:total_sizei{}ee", info.len()).into_bytes();
            response.extend(info);
            write_extended(&mut s, 1, &response).await.unwrap();
        });
        // Exact size is generated below to make the fake peer exercise the real wire protocol.
        let result = tokio::time::timeout(Duration::from_secs(2), metadata(addr, hash))
            .await
            .unwrap();
        if result.is_err() {
            peer.abort();
        } else {
            peer.await.unwrap();
        }
        assert!(result.is_ok(), "{result:?}");
    }
    #[tokio::test]
    async fn pause_waits_for_worker() {
        let db = Arc::new(Mutex::new(
            Storage::new(std::path::Path::new(":memory:")).unwrap(),
        ));
        let d = Discovery::new();
        d.start(db.clone()).await.unwrap();
        d.pause().await;
        assert!(!d.status().running);
        let before = db.lock().unwrap().count().unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(db.lock().unwrap().count().unwrap(), before);
    }
    #[tokio::test]
    #[ignore = "Consulta la red DHT pública durante 30 segundos"]
    async fn live_dht() {
        let db = Arc::new(Mutex::new(
            Storage::new(std::path::Path::new(":memory:")).unwrap(),
        ));
        let discovery = Discovery::new();
        discovery.start(db).await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        let status = discovery.status();
        println!(
            "DHT: nodos={}, hashes={}, metadatos={}, estado={}",
            status.nodes, status.hashes, status.indexed, status.message
        );
        discovery.pause().await;
        assert!(!discovery.status().running);
        assert!(
            status.nodes > 0,
            "No hubo respuesta de nodos DHT: {}",
            status.message
        );
    }
}
