//! Lecture tolérante des logs Minecraft (vanilla, Fabric, Forge, NeoForge, Paper)
//! et extraction des événements utiles (connexions, advancements, commandes admin, lag...).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl Level {
    fn parse(s: &str) -> Level {
        match s {
            "TRACE" => Level::Trace,
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARN" | "WARNING" => Level::Warn,
            "ERROR" | "SEVERE" => Level::Error,
            "FATAL" => Level::Fatal,
            _ => Level::Unknown,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "trace",
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
            Level::Fatal => "fatal",
            Level::Unknown => "unknown",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogLine {
    /// Heure "HH:MM:SS" extraite du log, vide sur une ligne de continuation.
    pub ts: String,
    pub level: Level,
    pub thread: String,
    pub logger: String,
    pub message: String,
    pub raw: String,
    /// Ligne sans en-tête (stacktrace, message multi-lignes).
    pub continuation: bool,
}

static HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[(?P<ts>[^\]]+)\] \[(?P<thread>[^/\]]*)/(?P<level>[A-Za-z]+)\](?: \[(?P<logger>[^\]]*)\])?:?\s?(?P<msg>.*)$")
        .unwrap()
});
static HEADER_PAPER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(?P<ts>\d{2}:\d{2}:\d{2}) (?P<level>[A-Za-z]+)\]:?\s?(?P<msg>.*)$").unwrap());
static TIME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{2}:\d{2}:\d{2})").unwrap());

pub fn parse_line(raw: &str) -> LogLine {
    let raw_trimmed = raw.trim_end_matches(['\r', '\n']);
    if let Some(c) = HEADER.captures(raw_trimmed) {
        return LogLine {
            ts: TIME.captures(&c["ts"]).map(|t| t[1].to_string()).unwrap_or_default(),
            level: Level::parse(&c["level"].to_ascii_uppercase()),
            thread: c["thread"].to_string(),
            logger: c.name("logger").map(|m| m.as_str().to_string()).unwrap_or_default(),
            message: c["msg"].to_string(),
            raw: raw_trimmed.to_string(),
            continuation: false,
        };
    }
    if let Some(c) = HEADER_PAPER.captures(raw_trimmed) {
        return LogLine {
            ts: c["ts"].to_string(),
            level: Level::parse(&c["level"].to_ascii_uppercase()),
            thread: String::new(),
            logger: String::new(),
            message: c["msg"].to_string(),
            raw: raw_trimmed.to_string(),
            continuation: false,
        };
    }
    LogLine {
        ts: String::new(),
        level: Level::Unknown,
        thread: String::new(),
        logger: String::new(),
        message: raw_trimmed.to_string(),
        raw: raw_trimmed.to_string(),
        continuation: true,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdminAction {
    GameMode { target: String, mode: String },
    Give { target: String, item: String, count: u64 },
    Op { target: String },
    Deop { target: String },
    Teleport { target: String, dest: String },
    Kill { target: String },
    Effect,
    Xp,
    Time,
    Weather,
    Difficulty,
    GameRule,
    Whitelist,
    Ban,
    Kick,
    Summon,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LogEvent {
    Joined { name: String, ip: String },
    Left { name: String, reason: String },
    Advancement { name: String, title: String },
    Chat { name: String, text: String },
    /// Retour d'une commande tapée par un joueur (gamerule logAdminCommands).
    AdminFeedback { actor: String, text: String, action: AdminAction },
    Lag { ms: u64, ticks: u64 },
    Starting { version: String },
    Progress { percent: u8 },
    Started { secs: f64 },
    Stopping,
    List { online: u32, max: u32, names: Vec<String> },
    Crash { path: String },
    MovedWrongly { name: String, text: String },
    RconReady,
}

macro_rules! pat {
    ($name:ident, $re:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($re).unwrap());
    };
}

pat!(RE_LOGIN, r"^(?P<name>[A-Za-z0-9_.]{1,40})(?:\[/(?P<ip>[^\]]*)\])? logged in with entity id");
pat!(RE_JOINED, r"^(?P<name>[A-Za-z0-9_.]{1,40}) joined the game$");
pat!(RE_LEFT, r"^(?P<name>[A-Za-z0-9_.]{1,40}) left the game$");
pat!(RE_LOST, r"^(?P<name>[A-Za-z0-9_.]{1,40}) lost connection: (?P<reason>.*)$");
pat!(RE_ADV, r"^(?P<name>[A-Za-z0-9_.]{1,40}) has (?:made the advancement|completed the challenge|reached the goal) \[(?P<title>[^\]]+)\]$");
pat!(RE_CHAT, r"^(?:\[Not Secure\] )?<(?P<name>[^>]{1,40})> (?P<text>.*)$");
pat!(RE_ADMIN, r"^\[(?P<actor>[^:\]]{1,40}): (?P<text>.*)\]$");
pat!(RE_LAG, r"Can't keep up! Is the server overloaded\? Running (?P<ms>\d+)ms or (?P<ticks>\d+) ticks behind");
pat!(RE_STARTING, r"^Starting minecraft server version (?P<ver>.+)$");
pat!(RE_PROGRESS, r"^Preparing spawn area: (?P<pct>\d+)%");
pat!(RE_DONE, r"^Done \((?P<secs>[\d.,]+)s\)!");
pat!(RE_STOPPING, r"^Stopping (?:the )?server$");
pat!(RE_LIST, r"^There are (?P<online>\d+)(?: of a max of |/)(?P<max>\d+) players online:?\s*(?P<names>.*)$");
pat!(RE_CRASH, r"(?:This crash report has been saved to|Crash report saved to):?\s*(?P<path>.+)$");
pat!(RE_MOVED, r"^(?P<name>[A-Za-z0-9_.]{1,40}) moved (?P<what>too quickly!|wrongly!)");
pat!(RE_RCON, r"^RCON running on ");

pat!(RE_GM_OWN, r"^Set own game mode to (?P<mode>.+?) Mode$");
pat!(RE_GM_OTHER, r"^Set (?P<target>.+?)'s game mode to (?P<mode>.+?) Mode$");
pat!(RE_GIVE, r"^Gave (?P<count>\d+) \[(?P<item>[^\]]+)\] to (?P<target>.+)$");
pat!(RE_OP, r"^Made (?P<target>.+?) a server operator$");
pat!(RE_DEOP, r"^Made (?P<target>.+?) no longer a server operator$");
pat!(RE_TP, r"^Teleported (?P<target>.+?) to (?P<dest>.+)$");
pat!(RE_KILL, r"^Killed (?P<target>.+)$");

pub fn extract_event(line: &LogLine) -> Option<LogEvent> {
    if line.continuation {
        return None;
    }
    let m = line.message.trim();
    if m.is_empty() {
        return None;
    }
    if let Some(c) = RE_LOGIN.captures(m) {
        return Some(LogEvent::Joined {
            name: c["name"].to_string(),
            ip: c.name("ip").map(|i| i.as_str().to_string()).unwrap_or_default(),
        });
    }
    if let Some(c) = RE_JOINED.captures(m) {
        return Some(LogEvent::Joined { name: c["name"].to_string(), ip: String::new() });
    }
    if let Some(c) = RE_LEFT.captures(m) {
        return Some(LogEvent::Left { name: c["name"].to_string(), reason: String::new() });
    }
    if let Some(c) = RE_LOST.captures(m) {
        return Some(LogEvent::Left { name: c["name"].to_string(), reason: c["reason"].to_string() });
    }
    if let Some(c) = RE_ADV.captures(m) {
        return Some(LogEvent::Advancement { name: c["name"].to_string(), title: c["title"].to_string() });
    }
    if let Some(c) = RE_CHAT.captures(m) {
        return Some(LogEvent::Chat { name: c["name"].to_string(), text: c["text"].to_string() });
    }
    if let Some(c) = RE_ADMIN.captures(m) {
        let actor = c["actor"].to_string();
        let text = c["text"].to_string();
        let action = classify_admin(&actor, &text);
        return Some(LogEvent::AdminFeedback { actor, text, action });
    }
    if let Some(c) = RE_LAG.captures(m) {
        return Some(LogEvent::Lag { ms: c["ms"].parse().unwrap_or(0), ticks: c["ticks"].parse().unwrap_or(0) });
    }
    if let Some(c) = RE_DONE.captures(m) {
        return Some(LogEvent::Started { secs: c["secs"].replace(',', ".").parse().unwrap_or(0.0) });
    }
    if let Some(c) = RE_PROGRESS.captures(m) {
        return Some(LogEvent::Progress { percent: c["pct"].parse().unwrap_or(0) });
    }
    if let Some(c) = RE_STARTING.captures(m) {
        return Some(LogEvent::Starting { version: c["ver"].to_string() });
    }
    if RE_STOPPING.is_match(m) {
        return Some(LogEvent::Stopping);
    }
    if let Some(c) = RE_LIST.captures(m) {
        let names = c["names"]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Some(LogEvent::List {
            online: c["online"].parse().unwrap_or(0),
            max: c["max"].parse().unwrap_or(0),
            names,
        });
    }
    if let Some(c) = RE_CRASH.captures(m) {
        return Some(LogEvent::Crash { path: c["path"].trim().to_string() });
    }
    if let Some(c) = RE_MOVED.captures(m) {
        return Some(LogEvent::MovedWrongly { name: c["name"].to_string(), text: m.to_string() });
    }
    if RE_RCON.is_match(m) {
        return Some(LogEvent::RconReady);
    }
    None
}

fn classify_admin(actor: &str, text: &str) -> AdminAction {
    if let Some(c) = RE_GM_OWN.captures(text) {
        return AdminAction::GameMode { target: actor.to_string(), mode: c["mode"].to_string() };
    }
    if let Some(c) = RE_GM_OTHER.captures(text) {
        return AdminAction::GameMode { target: c["target"].to_string(), mode: c["mode"].to_string() };
    }
    if let Some(c) = RE_GIVE.captures(text) {
        return AdminAction::Give {
            target: c["target"].to_string(),
            item: c["item"].to_string(),
            count: c["count"].parse().unwrap_or(0),
        };
    }
    if let Some(c) = RE_OP.captures(text) {
        return AdminAction::Op { target: c["target"].to_string() };
    }
    if let Some(c) = RE_DEOP.captures(text) {
        return AdminAction::Deop { target: c["target"].to_string() };
    }
    if let Some(c) = RE_TP.captures(text) {
        return AdminAction::Teleport { target: c["target"].to_string(), dest: c["dest"].to_string() };
    }
    if let Some(c) = RE_KILL.captures(text) {
        return AdminAction::Kill { target: c["target"].to_string() };
    }
    let t = text.to_ascii_lowercase();
    if t.starts_with("applied effect") || t.starts_with("removed effect") || t.contains(" effect") {
        AdminAction::Effect
    } else if t.contains("experience") {
        AdminAction::Xp
    } else if t.starts_with("set the time") || t.starts_with("added") && t.contains("to the time") {
        AdminAction::Time
    } else if t.starts_with("set the weather") || t.starts_with("changing to") {
        AdminAction::Weather
    } else if t.contains("difficulty") {
        AdminAction::Difficulty
    } else if t.starts_with("gamerule") || t.contains("is now set to") {
        AdminAction::GameRule
    } else if t.contains("whitelist") {
        AdminAction::Whitelist
    } else if t.starts_with("banned") || t.starts_with("unbanned") {
        AdminAction::Ban
    } else if t.starts_with("kicked") {
        AdminAction::Kick
    } else if t.starts_with("summoned") {
        AdminAction::Summon
    } else {
        AdminAction::Other
    }
}

/// Vrai pour une ligne de stacktrace Java.
pub fn is_stack_frame(raw: &str) -> bool {
    let t = raw.trim_start();
    t.starts_with("at ")
        || t.starts_with("Caused by:")
        || t.starts_with("... ")
        || t.starts_with("Suppressed:")
        || t.starts_with("Exception in thread")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vanilla_header() {
        let l = parse_line("[12:34:56] [Server thread/INFO]: Done (3.251s)! For help, type \"help\"\n");
        assert_eq!(l.ts, "12:34:56");
        assert_eq!(l.level, Level::Info);
        assert_eq!(l.thread, "Server thread");
        assert!(!l.continuation);
        assert_eq!(extract_event(&l), Some(LogEvent::Started { secs: 3.251 }));
    }

    #[test]
    fn parses_neoforge_header() {
        let l = parse_line("[06Oct2026 20:01:02.123] [Server thread/WARN] [net.minecraft.server.MinecraftServer/]: Can't keep up! Is the server overloaded? Running 2534ms or 50 ticks behind");
        assert_eq!(l.ts, "20:01:02");
        assert_eq!(l.level, Level::Warn);
        assert_eq!(l.logger, "net.minecraft.server.MinecraftServer/");
        assert_eq!(extract_event(&l), Some(LogEvent::Lag { ms: 2534, ticks: 50 }));
    }

    #[test]
    fn parses_paper_header_and_continuation() {
        let l = parse_line("[12:00:00 ERROR]: Something broke");
        assert_eq!(l.level, Level::Error);
        assert_eq!(l.message, "Something broke");
        let c = parse_line("\tat net.minecraft.server.MinecraftServer.tick(MinecraftServer.java:123)");
        assert!(c.continuation);
        assert!(is_stack_frame(&c.raw));
    }

    #[test]
    fn player_events() {
        let j = parse_line("[10:00:00] [Server thread/INFO]: Steve[/192.168.1.10:51234] logged in with entity id 123 at (1.0, 64.0, 2.0)");
        assert_eq!(extract_event(&j), Some(LogEvent::Joined { name: "Steve".into(), ip: "192.168.1.10:51234".into() }));
        let j2 = parse_line("[10:00:00] [Server thread/INFO]: Steve joined the game");
        assert_eq!(extract_event(&j2), Some(LogEvent::Joined { name: "Steve".into(), ip: String::new() }));
        let l = parse_line("[10:05:00] [Server thread/INFO]: Steve lost connection: Disconnected");
        assert_eq!(extract_event(&l), Some(LogEvent::Left { name: "Steve".into(), reason: "Disconnected".into() }));
        let a = parse_line("[10:06:00] [Server thread/INFO]: Steve has made the advancement [Diamonds!]");
        assert_eq!(extract_event(&a), Some(LogEvent::Advancement { name: "Steve".into(), title: "Diamonds!".into() }));
        let c = parse_line("[10:06:00] [Server thread/INFO]: [Not Secure] <Steve> salut");
        assert_eq!(extract_event(&c), Some(LogEvent::Chat { name: "Steve".into(), text: "salut".into() }));
    }

    #[test]
    fn admin_feedback() {
        let g = parse_line("[10:00:00] [Server thread/INFO]: [Steve: Set own game mode to Creative Mode]");
        match extract_event(&g) {
            Some(LogEvent::AdminFeedback { actor, action: AdminAction::GameMode { target, mode }, .. }) => {
                assert_eq!(actor, "Steve");
                assert_eq!(target, "Steve");
                assert_eq!(mode, "Creative");
            }
            other => panic!("inattendu: {other:?}"),
        }
        let g = parse_line("[10:00:00] [Server thread/INFO]: [Steve: Gave 64 [Diamond] to Alex]");
        match extract_event(&g) {
            Some(LogEvent::AdminFeedback { action: AdminAction::Give { target, item, count }, .. }) => {
                assert_eq!((target.as_str(), item.as_str(), count), ("Alex", "Diamond", 64));
            }
            other => panic!("inattendu: {other:?}"),
        }
        let g = parse_line("[10:00:00] [Server thread/INFO]: [Steve: Set the time to 1000]");
        match extract_event(&g) {
            Some(LogEvent::AdminFeedback { action: AdminAction::Time, .. }) => {}
            other => panic!("inattendu: {other:?}"),
        }
        let plain = parse_line("[10:00:00] [Server thread/INFO]: Set Steve's game mode to Creative Mode");
        assert_eq!(extract_event(&plain), None);
    }

    #[test]
    fn list_and_crash() {
        let l = parse_line("[10:00:00] [Server thread/INFO]: There are 2 of a max of 20 players online: Steve, Alex");
        assert_eq!(extract_event(&l), Some(LogEvent::List { online: 2, max: 20, names: vec!["Steve".into(), "Alex".into()] }));
        let c = parse_line("[10:00:00] [Server thread/ERROR]: This crash report has been saved to: C:\\srv\\crash-reports\\crash-2026.txt");
        assert_eq!(extract_event(&c), Some(LogEvent::Crash { path: "C:\\srv\\crash-reports\\crash-2026.txt".into() }));
    }
}
