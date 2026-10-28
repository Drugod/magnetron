//! Only individual public YouTube video URLs enter the video worker.
pub fn identity(text: &str) -> Result<(String, String), String> {
    if text.len() > 16384 || text.chars().any(char::is_control) {
        return Err("Enlace YouTube inválido".into());
    }
    let u = url::Url::parse(text).map_err(|_| "Enlace YouTube inválido")?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
    {
        return Err("YouTube requiere un enlace HTTPS sin credenciales".into());
    }
    let host = u.host_str().unwrap_or("");
    let id = match host {
        "youtu.be" => u.path().strip_prefix('/').unwrap_or("").to_owned(),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" => {
            if u.path() == "/watch" {
                let ids: Vec<_> = u
                    .query_pairs()
                    .filter(|(k, _)| k == "v")
                    .map(|(_, v)| v.into_owned())
                    .collect();
                if ids.len() != 1 {
                    return Err("Se necesita un vídeo individual de YouTube".into());
                }
                ids[0].clone()
            } else {
                u.path()
                    .strip_prefix("/shorts/")
                    .or_else(|| u.path().strip_prefix("/live/"))
                    .unwrap_or("")
                    .to_owned()
            }
        }
        _ => return Err("Enlace YouTube no admitido".into()),
    };
    if id.len() != 11
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("Se necesita un vídeo individual de YouTube".into());
    }
    Ok((
        format!("youtube:{id}"),
        format!("https://www.youtube.com/watch?v={id}"),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_single_videos_and_discards_tracking() {
        for u in [
            "https://youtu.be/abcdefgh_-1?si=tracking",
            "https://www.youtube.com/watch?v=abcdefgh_-1&list=tracking",
            "https://m.youtube.com/shorts/abcdefgh_-1",
        ] {
            assert_eq!(
                identity(u).unwrap(),
                (
                    "youtube:abcdefgh_-1".into(),
                    "https://www.youtube.com/watch?v=abcdefgh_-1".into()
                )
            );
        }
        for u in [
            "http://youtu.be/abcdefgh_-1",
            "https://youtube.com.evil.test/watch?v=abcdefgh_-1",
            "https://youtube.com@evil.test/watch?v=abcdefgh_-1",
            "https://youtube.com/playlist?list=x",
            "https://youtu.be/../../file",
            "https://youtube.com/watch?v=abcdefgh_-1&v=abcdefgh_-2",
            "file:///etc/passwd",
            "https://youtu.be/abc%2fdefgh_1",
        ] {
            assert!(identity(u).is_err(), "{u}");
        }
    }
}
