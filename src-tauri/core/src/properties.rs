//! Lecture minimale de server.properties.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn read(server_dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(text) = fs::read_to_string(server_dir.join("server.properties")) else { return map };
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

pub fn level_name(server_dir: &Path) -> String {
    read(server_dir).get("level-name").cloned().filter(|s| !s.is_empty()).unwrap_or_else(|| "world".into())
}

pub fn server_port(server_dir: &Path) -> Option<u16> {
    read(server_dir).get("server-port").and_then(|p| p.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_properties() {
        let dir = std::env::temp_dir().join(format!("panel-props-{}", crate::config::new_id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("server.properties"), "#Minecraft server properties\nlevel-name=monde\nserver-port=25566\nmotd=A Minecraft Server\n").unwrap();
        assert_eq!(level_name(&dir), "monde");
        assert_eq!(server_port(&dir), Some(25566));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn defaults_without_file() {
        assert_eq!(level_name(Path::new("/nonexistent/xyz")), "world");
    }
}
