//! Regroupement des erreurs des logs en "bugs" dédupliqués par empreinte.

use crate::logs::{is_stack_frame, Level, LogEvent, LogLine};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Instant;

const MAX_DETAIL_LINES: usize = 150;
const MAX_DETAIL_CHARS: usize = 12_000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BugReport {
    /// "error" | "warn" | "lag" | "crash"
    pub category: String,
    pub level: String,
    pub title: String,
    pub exception: String,
    pub details: String,
    pub fingerprint: String,
}

struct Pending {
    line: LogLine,
    lines: Vec<String>,
    started: Instant,
    qualified: bool,
}

/// Accumule les lignes d'un serveur et produit un BugReport quand une erreur
/// (et sa stacktrace) est complète.
pub struct ErrorCollector {
    pending: Option<Pending>,
}

impl Default for ErrorCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self { pending: None }
    }

    pub fn feed(&mut self, line: &LogLine, event: Option<&LogEvent>) -> Option<BugReport> {
        if line.continuation {
            if let Some(p) = self.pending.as_mut() {
                if is_stack_frame(&line.raw) {
                    p.qualified = true;
                }
                if p.lines.len() < MAX_DETAIL_LINES {
                    p.lines.push(line.raw.clone());
                }
            }
            return None;
        }
        let finished = self.finish();
        if let Some(LogEvent::Lag { ms, ticks }) = event {
            // Le lag est un bug "instantané", pas de stacktrace à attendre.
            let report = BugReport {
                category: "lag".into(),
                level: "warn".into(),
                title: "Le serveur ne suit plus (Can't keep up)".into(),
                exception: String::new(),
                details: format!("{} ms de retard, {} ticks", ms, ticks),
                fingerprint: "lag|cant-keep-up".into(),
            };
            return finished.or(Some(report));
        }
        if let Some(LogEvent::Crash { path }) = event {
            let report = BugReport {
                category: "crash".into(),
                level: "fatal".into(),
                title: "Crash du serveur".into(),
                exception: String::new(),
                details: format!("Rapport : {path}"),
                fingerprint: "crash|report".into(),
            };
            return finished.or(Some(report));
        }
        let qualified = matches!(line.level, Level::Error | Level::Fatal)
            || (line.level == Level::Warn && looks_like_exception(&line.message));
        // On garde la ligne courante en attente : une stacktrace qui suit la qualifie.
        self.pending = Some(Pending { line: line.clone(), lines: Vec::new(), started: Instant::now(), qualified });
        finished
    }

    /// À appeler périodiquement : clôt une erreur en attente sans nouvelle ligne.
    pub fn flush_if_idle(&mut self, idle_ms: u128) -> Option<BugReport> {
        let idle = self.pending.as_ref().map(|p| p.started.elapsed().as_millis() >= idle_ms).unwrap_or(false);
        if idle {
            self.finish()
        } else {
            None
        }
    }

    pub fn finish(&mut self) -> Option<BugReport> {
        let p = self.pending.take()?;
        if !p.qualified {
            return None;
        }
        let mut details = String::new();
        details.push_str(&p.line.raw);
        for l in &p.lines {
            if details.len() + l.len() + 1 > MAX_DETAIL_CHARS {
                details.push_str("\n…");
                break;
            }
            details.push('\n');
            details.push_str(l);
        }
        let exception = find_exception(&p.line.message)
            .or_else(|| p.lines.iter().find_map(|l| find_exception(l)))
            .unwrap_or_default();
        let title = truncate(&p.line.message, 300);
        let level = if matches!(p.line.level, Level::Error | Level::Fatal) { "error" } else { "warn" };
        let fingerprint = format!("{}|{}|{}", level, exception, truncate(&normalize(&p.line.message), 160));
        Some(BugReport {
            category: level.to_string(),
            level: p.line.level.as_str().to_string(),
            title,
            exception,
            details,
            fingerprint,
        })
    }
}

fn looks_like_exception(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("exception") || m.contains("error") || m.contains("failed to") || m.contains("unable to")
}

static RE_EXC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:[A-Za-z_$][\w$]*\.)+([A-Z][\w$]*(?:Exception|Error|Throwable))").unwrap());
static RE_UUID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}").unwrap());
static RE_HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"0x[0-9a-fA-F]+").unwrap());
static RE_NUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-?\d+(?:[.,]\d+)?").unwrap());
static RE_QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"]*"|'[^']*'"#).unwrap());
static RE_WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

pub fn find_exception(text: &str) -> Option<String> {
    RE_EXC.captures(text).map(|c| c[1].to_string())
}

/// Normalise un message pour que deux occurrences du même bug aient la même empreinte.
pub fn normalize(msg: &str) -> String {
    let s = RE_UUID.replace_all(msg, "<uuid>");
    let s = RE_HEX.replace_all(&s, "<hex>");
    let s = RE_QUOTED.replace_all(&s, "<q>");
    let s = RE_NUM.replace_all(&s, "#");
    let s = RE_WS.replace_all(&s, " ");
    s.trim().to_ascii_lowercase()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::{extract_event, parse_line};

    fn feed(c: &mut ErrorCollector, raw: &str) -> Option<BugReport> {
        let l = parse_line(raw);
        let ev = extract_event(&l);
        c.feed(&l, ev.as_ref())
    }

    #[test]
    fn groups_error_with_stacktrace() {
        let mut c = ErrorCollector::new();
        assert!(feed(&mut c, "[10:00:00] [Server thread/ERROR]: Exception ticking world").is_none());
        assert!(feed(&mut c, "java.lang.NullPointerException: Cannot invoke \"x\"").is_none());
        assert!(feed(&mut c, "\tat net.minecraft.world.level.Level.tick(Level.java:120)").is_none());
        let bug = feed(&mut c, "[10:00:01] [Server thread/INFO]: Steve joined the game").unwrap();
        assert_eq!(bug.category, "error");
        assert_eq!(bug.exception, "NullPointerException");
        assert!(bug.details.contains("Level.java:120"));
        assert!(bug.fingerprint.starts_with("error|NullPointerException|"));
    }

    #[test]
    fn warn_without_exception_is_ignored_unless_stacktrace() {
        let mut c = ErrorCollector::new();
        assert!(feed(&mut c, "[10:00:00] [main/WARN]: Mod foo uses deprecated API").is_none());
        assert!(feed(&mut c, "[10:00:01] [main/INFO]: ok").is_none());
        assert!(feed(&mut c, "[10:00:02] [main/WARN]: Something odd").is_none());
        assert!(feed(&mut c, "\tat foo.Bar.baz(Bar.java:1)").is_none());
        let bug = feed(&mut c, "[10:00:03] [main/INFO]: ok").unwrap();
        assert_eq!(bug.category, "warn");
        assert_eq!(bug.title, "Something odd");
    }

    #[test]
    fn same_error_different_numbers_same_fingerprint() {
        let a = normalize("Failed to save chunk at [12, -4] for player 3f2a… after 1500ms");
        let b = normalize("Failed to save chunk at [7, 99] for player 3f2a… after 2ms");
        assert_eq!(a, b);
    }

    #[test]
    fn lag_is_immediate() {
        let mut c = ErrorCollector::new();
        let bug = feed(&mut c, "[10:00:00] [Server thread/WARN]: Can't keep up! Is the server overloaded? Running 2000ms or 40 ticks behind").unwrap();
        assert_eq!(bug.category, "lag");
        assert_eq!(bug.fingerprint, "lag|cant-keep-up");
    }

    #[test]
    fn idle_flush() {
        let mut c = ErrorCollector::new();
        assert!(feed(&mut c, "[10:00:00] [Server thread/ERROR]: Boom").is_none());
        assert!(c.flush_if_idle(0).is_some());
        assert!(c.flush_if_idle(0).is_none());
    }
}
