//! Server List Ping : état d'un serveur Minecraft (joueurs, version, MOTD)
//! sans compte ni mod, sur le port de jeu.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PingResult {
    pub online: bool,
    pub players_online: u32,
    pub players_max: u32,
    pub version: String,
    pub protocol: i64,
    pub motd: String,
    pub latency_ms: u64,
    pub sample: Vec<String>,
    pub error: String,
}

pub fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if v == 0 {
            break;
        }
    }
}

pub fn read_varint<R: Read>(r: &mut R) -> std::io::Result<i32> {
    let mut result: i32 = 0;
    for i in 0..5 {
        let mut b = [0u8; 1];
        r.read_exact(&mut b)?;
        result |= ((b[0] & 0x7F) as i32) << (7 * i);
        if b[0] & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "varint trop long"))
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}

fn frame(packet: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(packet.len() + 5);
    write_varint(&mut out, packet.len() as i32);
    out.extend_from_slice(packet);
    out
}

pub fn ping(host: &str, port: u16, timeout: Duration) -> PingResult {
    match ping_inner(host, port, timeout) {
        Ok(r) => r,
        Err(e) => PingResult { online: false, error: e, ..Default::default() },
    }
}

fn ping_inner(host: &str, port: u16, timeout: Duration) -> Result<PingResult, String> {
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("Résolution DNS : {e}"))?
        .next()
        .ok_or_else(|| "Adresse introuvable".to_string())?;
    let start = Instant::now();
    let mut stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("Connexion : {e}"))?;
    stream.set_read_timeout(Some(timeout)).ok();
    stream.set_write_timeout(Some(timeout)).ok();

    let mut hs = Vec::new();
    write_varint(&mut hs, 0x00);
    write_varint(&mut hs, 767);
    write_string(&mut hs, host);
    hs.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut hs, 1);
    stream.write_all(&frame(&hs)).map_err(|e| format!("Handshake : {e}"))?;
    stream.write_all(&frame(&[0x00])).map_err(|e| format!("Requête status : {e}"))?;

    let len = read_varint(&mut stream).map_err(|e| format!("Réponse : {e}"))?;
    if !(2..=1_000_000).contains(&len) {
        return Err("Réponse invalide".into());
    }
    let mut packet = vec![0u8; len as usize];
    stream.read_exact(&mut packet).map_err(|e| format!("Lecture réponse : {e}"))?;
    let latency_ms = start.elapsed().as_millis() as u64;
    let mut cursor = std::io::Cursor::new(&packet);
    let id = read_varint(&mut cursor).map_err(|e| e.to_string())?;
    if id != 0 {
        return Err(format!("Paquet inattendu ({id})"));
    }
    let slen = read_varint(&mut cursor).map_err(|e| e.to_string())? as usize;
    let pos = cursor.position() as usize;
    let json = packet.get(pos..pos + slen).ok_or_else(|| "Réponse tronquée".to_string())?;
    let json = std::str::from_utf8(json).map_err(|_| "Réponse non UTF-8".to_string())?;
    let mut r = parse_status(json)?;
    r.latency_ms = latency_ms;
    Ok(r)
}

pub fn parse_status(json: &str) -> Result<PingResult, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("JSON status : {e}"))?;
    let players = v.get("players");
    let sample = players
        .and_then(|p| p.get("sample"))
        .and_then(|s| s.as_array())
        .map(|arr| arr.iter().filter_map(|e| e.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
        .unwrap_or_default();
    Ok(PingResult {
        online: true,
        players_online: players.and_then(|p| p.get("online")).and_then(|n| n.as_u64()).unwrap_or(0) as u32,
        players_max: players.and_then(|p| p.get("max")).and_then(|n| n.as_u64()).unwrap_or(0) as u32,
        version: v.get("version").and_then(|x| x.get("name")).and_then(|n| n.as_str()).unwrap_or("").to_string(),
        protocol: v.get("version").and_then(|x| x.get("protocol")).and_then(|n| n.as_i64()).unwrap_or(0),
        motd: v.get("description").map(chat_to_text).unwrap_or_default(),
        latency_ms: 0,
        sample,
        error: String::new(),
    })
}

/// Aplatit un composant de chat (texte brut ou objet {text, extra}) en texte simple.
pub fn chat_to_text(v: &serde_json::Value) -> String {
    let mut out = String::new();
    fn walk(v: &serde_json::Value, out: &mut String) {
        match v {
            serde_json::Value::String(s) => out.push_str(s),
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            serde_json::Value::Object(o) => {
                if let Some(t) = o.get("text").and_then(|t| t.as_str()) {
                    out.push_str(t);
                }
                if let Some(e) = o.get("extra") {
                    walk(e, out);
                }
            }
            _ => {}
        }
    }
    walk(v, &mut out);
    strip_color_codes(&out)
}

pub fn strip_color_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn varint_roundtrip() {
        for v in [0, 1, 127, 128, 255, 300, 25565, 767, i32::MAX, -1] {
            let mut buf = Vec::new();
            write_varint(&mut buf, v);
            let mut cur = std::io::Cursor::new(buf);
            assert_eq!(read_varint(&mut cur).unwrap(), v);
        }
    }

    #[test]
    fn parses_status_json() {
        let json = r#"{"version":{"name":"1.21.1","protocol":767},"players":{"max":20,"online":2,"sample":[{"name":"Steve","id":"x"}]},"description":{"text":"§aMon ","extra":[{"text":"serveur"}]}}"#;
        let r = parse_status(json).unwrap();
        assert_eq!(r.players_online, 2);
        assert_eq!(r.players_max, 20);
        assert_eq!(r.motd, "Mon serveur");
        assert_eq!(r.sample, vec!["Steve"]);
        let r2 = parse_status(r#"{"description":"plain","players":{"max":5,"online":0},"version":{"name":"x","protocol":1}}"#).unwrap();
        assert_eq!(r2.motd, "plain");
    }

    #[test]
    fn pings_fake_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            // handshake + status request
            let len = read_varint(&mut s).unwrap();
            let mut hs = vec![0u8; len as usize];
            s.read_exact(&mut hs).unwrap();
            let len = read_varint(&mut s).unwrap();
            let mut req = vec![0u8; len as usize];
            s.read_exact(&mut req).unwrap();
            let json = r#"{"version":{"name":"1.21.1","protocol":767},"players":{"max":10,"online":3},"description":"ok"}"#;
            let mut p = Vec::new();
            write_varint(&mut p, 0);
            write_string(&mut p, json);
            s.write_all(&frame(&p)).unwrap();
        });
        let r = ping("127.0.0.1", port, Duration::from_secs(3));
        assert!(r.online, "{}", r.error);
        assert_eq!(r.players_online, 3);
        assert_eq!(r.version, "1.21.1");
    }

    #[test]
    fn offline_server_reports_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let r = ping("127.0.0.1", port, Duration::from_millis(500));
        assert!(!r.online);
        assert!(!r.error.is_empty());
    }
}
