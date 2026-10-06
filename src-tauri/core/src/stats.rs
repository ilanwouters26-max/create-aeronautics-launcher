//! Lecture des statistiques joueurs (world/stats/<uuid>.json) et évaluation
//! des taux de minage par heure de jeu.

use crate::config::DetectionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerStats {
    pub uuid: String,
    pub name: String,
    pub play_hours: f64,
    /// Blocs minés par identifiant ("minecraft:diamond_ore" → 12).
    pub mined: HashMap<String, u64>,
    pub deaths: u64,
    pub player_kills: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MiningAlert {
    pub player: String,
    pub rule: String,
    pub count: u64,
    pub per_hour: f64,
    pub threshold: f64,
    pub play_hours: f64,
}

/// usercache.json : [{"name":"Steve","uuid":"...","expiresOn":"..."}]
pub fn read_usercache(server_dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(text) = fs::read_to_string(server_dir.join("usercache.json")) else { return map };
    let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else { return map };
    for e in entries {
        if let (Some(name), Some(uuid)) = (e.get("name").and_then(|v| v.as_str()), e.get("uuid").and_then(|v| v.as_str())) {
            map.insert(uuid.to_ascii_lowercase(), name.to_string());
        }
    }
    map
}

pub fn read_player_stats(server_dir: &Path, level_name: &str) -> Vec<PlayerStats> {
    let names = read_usercache(server_dir);
    let dir = server_dir.join(level_name).join("stats");
    let Ok(rd) = fs::read_dir(&dir) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let uuid = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
        let Ok(text) = fs::read_to_string(&path) else { continue };
        if let Some(stats) = parse_stats(&uuid, &text) {
            let name = names.get(&uuid).cloned().unwrap_or_else(|| uuid.clone());
            out.push(PlayerStats { name, ..stats });
        }
    }
    out.sort_by(|a, b| b.play_hours.partial_cmp(&a.play_hours).unwrap_or(std::cmp::Ordering::Equal));
    out
}

pub fn parse_stats(uuid: &str, text: &str) -> Option<PlayerStats> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let stats = v.get("stats")?;
    let custom = stats.get("minecraft:custom");
    let ticks = custom
        .and_then(|c| c.get("minecraft:play_time").or_else(|| c.get("minecraft:play_one_minute")))
        .and_then(|t| t.as_u64())
        .unwrap_or(0);
    let mut mined = HashMap::new();
    if let Some(obj) = stats.get("minecraft:mined").and_then(|m| m.as_object()) {
        for (k, val) in obj {
            if let Some(n) = val.as_u64() {
                mined.insert(k.clone(), n);
            }
        }
    }
    let deaths = custom.and_then(|c| c.get("minecraft:deaths")).and_then(|t| t.as_u64()).unwrap_or(0);
    let player_kills = custom.and_then(|c| c.get("minecraft:player_kills")).and_then(|t| t.as_u64()).unwrap_or(0);
    Some(PlayerStats {
        uuid: uuid.to_string(),
        name: uuid.to_string(),
        play_hours: ticks as f64 / 72_000.0,
        mined,
        deaths,
        player_kills,
    })
}

pub fn evaluate_mining(stats: &[PlayerStats], cfg: &DetectionConfig) -> Vec<MiningAlert> {
    let mut alerts = Vec::new();
    if !cfg.enabled || !cfg.mining {
        return alerts;
    }
    let min_hours = cfg.mining_min_playtime_min as f64 / 60.0;
    for p in stats {
        if p.play_hours < min_hours || p.play_hours <= 0.0 {
            continue;
        }
        for rule in &cfg.mining_rules {
            let count: u64 = rule.blocks.iter().map(|b| p.mined.get(b).copied().unwrap_or(0)).sum();
            let per_hour = count as f64 / p.play_hours;
            if count > 0 && per_hour > rule.per_hour {
                alerts.push(MiningAlert {
                    player: p.name.clone(),
                    rule: rule.label.clone(),
                    count,
                    per_hour: (per_hour * 10.0).round() / 10.0,
                    threshold: rule.per_hour,
                    play_hours: (p.play_hours * 100.0).round() / 100.0,
                });
            }
        }
    }
    alerts
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"stats":{"minecraft:mined":{"minecraft:diamond_ore":30,"minecraft:deepslate_diamond_ore":90,"minecraft:stone":5000},"minecraft:custom":{"minecraft:play_time":144000,"minecraft:deaths":2}},"DataVersion":3953}"#;

    #[test]
    fn parses_play_time_and_mined() {
        let s = parse_stats("abc", SAMPLE).unwrap();
        assert_eq!(s.play_hours, 2.0);
        assert_eq!(s.mined["minecraft:diamond_ore"], 30);
        assert_eq!(s.deaths, 2);
    }

    #[test]
    fn flags_high_diamond_rate() {
        let s = parse_stats("abc", SAMPLE).unwrap();
        let alerts = evaluate_mining(&[s.clone()], &DetectionConfig::default());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule, "Diamants");
        assert_eq!(alerts[0].count, 120);
        assert_eq!(alerts[0].per_hour, 60.0);
        assert!(evaluate_mining(&[s], &DetectionConfig::mining_event_preset()).is_empty());
    }

    #[test]
    fn respects_min_playtime() {
        let text = r#"{"stats":{"minecraft:mined":{"minecraft:diamond_ore":10},"minecraft:custom":{"minecraft:play_time":7200}}}"#;
        let s = parse_stats("x", text).unwrap();
        assert!(evaluate_mining(&[s], &DetectionConfig::default()).is_empty());
    }

    #[test]
    fn reads_stats_dir_with_usercache() {
        let dir = std::env::temp_dir().join(format!("panel-stats-{}", crate::config::new_id()));
        fs::create_dir_all(dir.join("world/stats")).unwrap();
        fs::write(dir.join("usercache.json"), r#"[{"name":"Steve","uuid":"11111111-1111-1111-1111-111111111111","expiresOn":"x"}]"#).unwrap();
        fs::write(dir.join("world/stats/11111111-1111-1111-1111-111111111111.json"), SAMPLE).unwrap();
        let stats = read_player_stats(&dir, "world");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].name, "Steve");
        let _ = fs::remove_dir_all(dir);
    }
}
