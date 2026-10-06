//! Client RCON (protocole Source, tel qu'implémenté par Minecraft) pour la
//! console des serveurs distants.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub const TYPE_RESPONSE: i32 = 0;
pub const TYPE_COMMAND: i32 = 2;
pub const TYPE_AUTH: i32 = 3;
const MAX_BODY: usize = 4096;

pub struct RconClient {
    stream: TcpStream,
    next_id: i32,
}

pub fn encode_packet(id: i32, kind: i32, body: &str) -> Vec<u8> {
    let len = 4 + 4 + body.len() + 2;
    let mut out = Vec::with_capacity(len + 4);
    out.extend_from_slice(&(len as i32).to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(body.as_bytes());
    out.push(0);
    out.push(0);
    out
}

pub fn read_packet<R: Read>(r: &mut R) -> std::io::Result<(i32, i32, String)> {
    let mut len_b = [0u8; 4];
    r.read_exact(&mut len_b)?;
    let len = i32::from_le_bytes(len_b);
    if !(10..=4096 + 10).contains(&len) {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("taille RCON invalide : {len}")));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    let id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let kind = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    let body = &buf[8..buf.len().saturating_sub(2)];
    Ok((id, kind, String::from_utf8_lossy(body).to_string()))
}

impl RconClient {
    pub fn connect(host: &str, port: u16, password: &str, timeout: Duration) -> Result<Self, String> {
        let addr = (host, port)
            .to_socket_addrs()
            .map_err(|e| format!("Résolution DNS : {e}"))?
            .next()
            .ok_or_else(|| "Adresse introuvable".to_string())?;
        let stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("Connexion RCON : {e}"))?;
        stream.set_read_timeout(Some(timeout)).ok();
        stream.set_write_timeout(Some(timeout)).ok();
        let mut client = Self { stream, next_id: 1 };
        let id = client.send(TYPE_AUTH, password)?;
        loop {
            let (rid, kind, _) = read_packet(&mut client.stream).map_err(|e| format!("Réponse auth : {e}"))?;
            if kind == TYPE_RESPONSE {
                continue;
            }
            if rid == -1 {
                return Err("Mot de passe RCON refusé".into());
            }
            if rid == id {
                return Ok(client);
            }
        }
    }

    fn send(&mut self, kind: i32, body: &str) -> Result<i32, String> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.stream.write_all(&encode_packet(id, kind, body)).map_err(|e| format!("Envoi RCON : {e}"))?;
        Ok(id)
    }

    pub fn command(&mut self, cmd: &str) -> Result<String, String> {
        let id = self.send(TYPE_COMMAND, cmd)?;
        let mut out = String::new();
        loop {
            let (rid, _, body) = read_packet(&mut self.stream).map_err(|e| format!("Réponse RCON : {e}"))?;
            if rid != id {
                continue;
            }
            let full = body.len() >= MAX_BODY;
            out.push_str(&body);
            if !full {
                break;
            }
        }
        Ok(crate::ping::strip_color_codes(&out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn packet_roundtrip() {
        let bytes = encode_packet(7, TYPE_COMMAND, "list");
        assert_eq!(bytes.len(), 4 + 4 + 4 + 4 + 2);
        let mut cur = std::io::Cursor::new(bytes);
        let (id, kind, body) = read_packet(&mut cur).unwrap();
        assert_eq!((id, kind, body.as_str()), (7, TYPE_COMMAND, "list"));
    }

    #[test]
    fn talks_to_fake_rcon_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let (id, kind, body) = read_packet(&mut s).unwrap();
            assert_eq!(kind, TYPE_AUTH);
            let reply_id = if body == "secret" { id } else { -1 };
            s.write_all(&encode_packet(reply_id, 2, "")).unwrap();
            if reply_id == -1 {
                return;
            }
            let (id, kind, body) = read_packet(&mut s).unwrap();
            assert_eq!(kind, TYPE_COMMAND);
            assert_eq!(body, "list");
            s.write_all(&encode_packet(id, TYPE_RESPONSE, "§aThere are 1 of a max of 20 players online: Steve")).unwrap();
        });
        let mut c = RconClient::connect("127.0.0.1", port, "secret", Duration::from_secs(3)).unwrap();
        let r = c.command("list").unwrap();
        assert_eq!(r, "There are 1 of a max of 20 players online: Steve");
    }

    #[test]
    fn rejects_bad_password() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = read_packet(&mut s).unwrap();
            s.write_all(&encode_packet(-1, 2, "")).unwrap();
        });
        let err = RconClient::connect("127.0.0.1", port, "wrong", Duration::from_secs(3)).err().unwrap();
        assert!(err.contains("refusé"));
    }
}
