//! Détection d'actions suspectes à partir des événements de log, selon la
//! configuration de détection propre à chaque serveur.

use crate::config::DetectionConfig;
use crate::logs::{AdminAction, LogEvent};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    High,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::High => "high",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SuspiciousEvent {
    pub severity: Severity,
    pub player: String,
    /// "gamemode" | "give" | "op" | "teleport" | "admin" | "early_advancement" | "mining" | "movement"
    pub category: String,
    pub title: String,
    pub details: String,
}

const CONSOLE_ACTORS: [&str; 2] = ["Server", "Rcon"];

pub struct Tracker {
    cfg: DetectionConfig,
    first_join: HashMap<String, DateTime<Local>>,
    last_move_warn: HashMap<String, DateTime<Local>>,
}

impl Tracker {
    pub fn new(cfg: DetectionConfig, known_first_join: HashMap<String, DateTime<Local>>) -> Self {
        Self { cfg, first_join: known_first_join, last_move_warn: HashMap::new() }
    }

    pub fn set_config(&mut self, cfg: DetectionConfig) {
        self.cfg = cfg;
    }

    pub fn first_join(&self, name: &str) -> Option<DateTime<Local>> {
        self.first_join.get(name).copied()
    }

    pub fn on_event(&mut self, ev: &LogEvent, now: DateTime<Local>) -> Option<SuspiciousEvent> {
        if let LogEvent::Joined { name, .. } = ev {
            self.first_join.entry(name.clone()).or_insert(now);
        }
        if !self.cfg.enabled {
            return None;
        }
        match ev {
            LogEvent::AdminFeedback { actor, text, action } => {
                if CONSOLE_ACTORS.contains(&actor.as_str()) {
                    return None;
                }
                self.admin(actor, text, action)
            }
            LogEvent::Advancement { name, title } => self.advancement(name, title, now),
            LogEvent::MovedWrongly { name, text } => {
                let recent = self
                    .last_move_warn
                    .get(name)
                    .map(|t| (now - *t).num_minutes() < 10)
                    .unwrap_or(false);
                if recent {
                    return None;
                }
                self.last_move_warn.insert(name.clone(), now);
                Some(SuspiciousEvent {
                    severity: Severity::Info,
                    player: name.clone(),
                    category: "movement".into(),
                    title: format!("{name} : déplacement anormal signalé par le serveur"),
                    details: format!("{text}. Peut venir d'un lag réseau comme d'un speed/fly."),
                })
            }
            _ => None,
        }
    }

    fn admin(&self, actor: &str, text: &str, action: &AdminAction) -> Option<SuspiciousEvent> {
        let c = &self.cfg;
        let mk = |severity, category: &str, title: String| SuspiciousEvent {
            severity,
            player: actor.to_string(),
            category: category.into(),
            title,
            details: text.to_string(),
        };
        match action {
            AdminAction::GameMode { target, mode } if c.gamemode => {
                let sev = if mode.eq_ignore_ascii_case("creative") || mode.eq_ignore_ascii_case("spectator") {
                    Severity::High
                } else {
                    Severity::Warn
                };
                let who = if target == actor { "lui-même".to_string() } else { target.clone() };
                Some(mk(sev, "gamemode", format!("{actor} a passé {who} en {mode}")))
            }
            AdminAction::Give { target, item, count } if c.give => {
                Some(mk(Severity::High, "give", format!("{actor} s'est give {count} × {item} (pour {target})")))
            }
            AdminAction::Op { target } if c.op => Some(mk(Severity::High, "op", format!("{actor} a mis {target} opérateur"))),
            AdminAction::Deop { target } if c.op => {
                Some(mk(Severity::High, "op", format!("{actor} a retiré l'op de {target}")))
            }
            AdminAction::Teleport { target, dest } if c.teleport => {
                Some(mk(Severity::Warn, "teleport", format!("{actor} a téléporté {target} vers {dest}")))
            }
            AdminAction::Kill { target } if c.other_admin => {
                Some(mk(Severity::Warn, "admin", format!("{actor} a tué {target}")))
            }
            AdminAction::GameMode { .. }
            | AdminAction::Give { .. }
            | AdminAction::Op { .. }
            | AdminAction::Deop { .. }
            | AdminAction::Teleport { .. }
            | AdminAction::Kill { .. } => None,
            _ if c.other_admin => Some(mk(Severity::Info, "admin", format!("{actor} : {text}"))),
            _ => None,
        }
    }

    fn advancement(&self, name: &str, title: &str, now: DateTime<Local>) -> Option<SuspiciousEvent> {
        if !self.cfg.early_advancements {
            return None;
        }
        let rule = self.cfg.early_rules.iter().find(|r| r.title.eq_ignore_ascii_case(title))?;
        let first = self.first_join.get(name)?;
        let minutes = (now - *first).num_minutes().max(0) as u64;
        if minutes > rule.max_minutes {
            return None;
        }
        Some(SuspiciousEvent {
            severity: Severity::Warn,
            player: name.to_string(),
            category: "early_advancement".into(),
            title: format!("{name} : « {title} » {minutes} min après sa première connexion"),
            details: format!("Seuil : {} min. Première connexion vue le {}.", rule.max_minutes, first.format("%d/%m %H:%M")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn gm(actor: &str, mode: &str) -> LogEvent {
        LogEvent::AdminFeedback {
            actor: actor.into(),
            text: format!("Set own game mode to {mode} Mode"),
            action: AdminAction::GameMode { target: actor.into(), mode: mode.into() },
        }
    }

    #[test]
    fn flags_creative_as_high_and_ignores_console() {
        let mut t = Tracker::new(DetectionConfig::default(), HashMap::new());
        let now = Local::now();
        let ev = t.on_event(&gm("Steve", "Creative"), now).unwrap();
        assert_eq!(ev.severity, Severity::High);
        assert_eq!(ev.category, "gamemode");
        assert!(t.on_event(&gm("Server", "Creative"), now).is_none());
        assert!(t.on_event(&gm("Rcon", "Creative"), now).is_none());
    }

    #[test]
    fn early_diamonds_uses_first_join() {
        let mut t = Tracker::new(DetectionConfig::default(), HashMap::new());
        let t0 = Local::now();
        t.on_event(&LogEvent::Joined { name: "Steve".into(), ip: String::new() }, t0);
        let ev = t
            .on_event(&LogEvent::Advancement { name: "Steve".into(), title: "Diamonds!".into() }, t0 + Duration::minutes(5))
            .unwrap();
        assert_eq!(ev.category, "early_advancement");
        // Trop tard pour être suspect.
        assert!(t
            .on_event(&LogEvent::Advancement { name: "Steve".into(), title: "Diamonds!".into() }, t0 + Duration::minutes(45))
            .is_none());
        // Joueur inconnu : pas de référence, pas d'alerte.
        assert!(t
            .on_event(&LogEvent::Advancement { name: "Alex".into(), title: "Diamonds!".into() }, t0)
            .is_none());
    }

    #[test]
    fn mining_event_preset_is_quiet_on_advancements_but_not_on_give() {
        let mut t = Tracker::new(DetectionConfig::mining_event_preset(), HashMap::new());
        let now = Local::now();
        t.on_event(&LogEvent::Joined { name: "Steve".into(), ip: String::new() }, now);
        assert!(t
            .on_event(&LogEvent::Advancement { name: "Steve".into(), title: "Diamonds!".into() }, now)
            .is_none());
        let give = LogEvent::AdminFeedback {
            actor: "Steve".into(),
            text: "Gave 64 [Diamond] to Steve".into(),
            action: AdminAction::Give { target: "Steve".into(), item: "Diamond".into(), count: 64 },
        };
        assert_eq!(t.on_event(&give, now).unwrap().severity, Severity::High);
    }

    #[test]
    fn movement_is_throttled() {
        let mut t = Tracker::new(DetectionConfig::default(), HashMap::new());
        let now = Local::now();
        let ev = LogEvent::MovedWrongly { name: "Steve".into(), text: "Steve moved too quickly!".into() };
        assert!(t.on_event(&ev, now).is_some());
        assert!(t.on_event(&ev, now + Duration::minutes(2)).is_none());
        assert!(t.on_event(&ev, now + Duration::minutes(20)).is_some());
    }
}
