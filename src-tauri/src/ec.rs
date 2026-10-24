//! Bounded, fixed-width aMule External Connections protocol client. Loopback only.
use md5::{Digest, Md5};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Clone, Debug)]
pub struct Tag {
    pub id: u16,
    pub kind: u8,
    pub data: Vec<u8>,
    pub children: Vec<Tag>,
}
impl Tag {
    pub fn string(id: u16, s: &str) -> Self {
        let mut data = s.as_bytes().to_vec();
        data.push(0);
        Self {
            id,
            kind: 6,
            data,
            children: vec![],
        }
    }
    pub fn number(id: u16, n: u64) -> Self {
        Self {
            id,
            kind: 5,
            data: n.to_be_bytes().to_vec(),
            children: vec![],
        }
    }
    pub fn hash(id: u16, data: Vec<u8>) -> Self {
        Self {
            id,
            kind: 9,
            data,
            children: vec![],
        }
    }
    pub fn get(&self, id: u16) -> Option<&Tag> {
        self.children.iter().find(|t| t.id == id)
    }
    pub fn num(&self) -> u64 {
        if !matches!(self.kind, 2..=5) || self.data.len() > 8 {
            return 0;
        }
        self.data.iter().fold(0, |n, b| (n << 8) | *b as u64)
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(self.data.strip_suffix(&[0]).unwrap_or(&self.data)).into_owned()
    }
    pub fn value(&self, id: u16) -> u64 {
        self.get(id).map_or(0, Tag::num)
    }
    fn encode(&self, out: &mut Vec<u8>) {
        let mut children = vec![];
        for c in &self.children {
            c.encode(&mut children)
        }
        out.extend_from_slice(
            &((self.id << 1) | u16::from(!self.children.is_empty())).to_be_bytes(),
        );
        out.push(self.kind);
        out.extend_from_slice(&((self.data.len() + children.len()) as u32).to_be_bytes());
        if !self.children.is_empty() {
            out.extend_from_slice(&(self.children.len() as u16).to_be_bytes());
            out.extend(children)
        }
        out.extend_from_slice(&self.data);
    }
}
fn take<'a>(data: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
    if data.len() < n {
        return Err("Respuesta eD2k truncada".into());
    }
    let (a, b) = data.split_at(n);
    *data = b;
    Ok(a)
}
fn tags(
    data: &mut &[u8],
    count: usize,
    depth: usize,
    budget: &mut usize,
) -> Result<Vec<Tag>, String> {
    if depth > 12 || count > *budget {
        return Err("Respuesta eD2k demasiado compleja".into());
    }
    *budget -= count;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let h = take(data, 7)?;
        let name = u16::from_be_bytes([h[0], h[1]]);
        let kind = h[2];
        let len = u32::from_be_bytes(h[3..7].try_into().unwrap()) as usize;
        if len > 4 * 1024 * 1024 || kind > 10 {
            return Err("Etiqueta eD2k inválida".into());
        }
        let mut children = vec![];
        let mut children_len = 0;
        if name & 1 != 0 {
            let n = take(data, 2)?;
            let count = u16::from_be_bytes([n[0], n[1]]) as usize;
            let before = data.len();
            children = tags(data, count, depth + 1, budget)?;
            children_len = before - data.len();
        }
        let value_len = len
            .checked_sub(children_len)
            .ok_or("Longitud eD2k inválida")?;
        result.push(Tag {
            id: name >> 1,
            kind,
            data: take(data, value_len)?.to_vec(),
            children,
        });
    }
    Ok(result)
}
pub struct Client(TcpStream);
impl Client {
    pub async fn connect(port: u16, password_hash: &str) -> Result<Self, String> {
        let stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|e| e.to_string())?;
        let mut c = Self(stream);
        let (op, t) = c
            .request(
                2,
                vec![
                    Tag::string(0x100, "MAGNETRON"),
                    Tag::string(0x101, env!("CARGO_PKG_VERSION")),
                    Tag::number(2, 0x204),
                ],
            )
            .await?;
        if op != 0x4f {
            return Err("El motor eD2k rechazó la conexión".into());
        }
        let salt = t
            .iter()
            .find(|t| t.id == 0xb)
            .ok_or("Falta el reto eD2k")?
            .num();
        let salt_hash = hex::encode(Md5::digest(format!("{salt:X}").as_bytes()));
        let hash =
            Md5::digest(format!("{}{salt_hash}", password_hash.to_lowercase()).as_bytes()).to_vec();
        if c.request(0x50, vec![Tag::hash(1, hash)]).await?.0 != 4 {
            return Err("No se pudo autenticar con el motor eD2k".into());
        }
        Ok(c)
    }
    pub async fn request(&mut self, op: u8, ts: Vec<Tag>) -> Result<(u8, Vec<Tag>), String> {
        tokio::time::timeout(Duration::from_secs(10), self.exchange(op, ts))
            .await
            .map_err(|_| "El motor eD2k no responde".to_string())?
    }
    async fn exchange(&mut self, op: u8, ts: Vec<Tag>) -> Result<(u8, Vec<Tag>), String> {
        let mut payload = vec![op];
        payload.extend_from_slice(&(ts.len() as u16).to_be_bytes());
        for t in ts {
            t.encode(&mut payload)
        }
        self.0.write_u32(0x20).await.map_err(|e| e.to_string())?;
        self.0
            .write_u32(payload.len() as u32)
            .await
            .map_err(|e| e.to_string())?;
        self.0
            .write_all(&payload)
            .await
            .map_err(|e| e.to_string())?;
        let flags = self.0.read_u32().await.map_err(|e| e.to_string())?;
        let len = self.0.read_u32().await.map_err(|e| e.to_string())? as usize;
        if flags != 0x20 || !(3..=4 * 1024 * 1024).contains(&len) {
            return Err("Cabecera eD2k inválida".into());
        }
        let mut payload = vec![0; len];
        self.0
            .read_exact(&mut payload)
            .await
            .map_err(|e| e.to_string())?;
        let op = payload[0];
        let count = u16::from_be_bytes([payload[1], payload[2]]) as usize;
        let mut body = &payload[3..];
        let tags = tags(&mut body, count, 0, &mut 20000)?;
        if !body.is_empty() {
            return Err("Datos sobrantes en respuesta eD2k".into());
        }
        if op == 5 {
            return Err(tags
                .iter()
                .find(|t| t.id == 0)
                .map(Tag::text)
                .unwrap_or_else(|| "El motor eD2k rechazó la operación".into()));
        }
        Ok((op, tags))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_roundtrip_and_bounds() {
        let t = Tag {
            id: 0x300,
            kind: 9,
            data: vec![42; 16],
            children: vec![Tag::string(0x301, "prueba"), Tag::number(0x306, 123456)],
        };
        let mut bytes = vec![];
        t.encode(&mut bytes);
        let mut input = &bytes[..];
        let parsed = tags(&mut input, 1, 0, &mut 20).unwrap();
        assert!(input.is_empty());
        assert_eq!(parsed[0].data, t.data);
        assert_eq!(parsed[0].value(0x306), 123456);
        assert!(tags(&mut &bytes[..bytes.len() - 1], 1, 0, &mut 20).is_err());
        assert!(tags(&mut &bytes[..], 1, 0, &mut 0).is_err());
    }
}
