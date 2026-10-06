//! Historique persistant (SQLite) : bugs regroupés, actions suspectes,
//! présence des joueurs, publications.

use crate::bugs::BugReport;
use crate::publish::PublishReport;
use crate::suspicious::SuspiciousEvent;
use chrono::{DateTime, Local};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub struct History {
    conn: Connection,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BugRow {
    pub id: i64,
    pub server_id: String,
    pub category: String,
    pub level: String,
    pub title: String,
    pub exception: String,
    pub details: String,
    pub count: i64,
    pub first_seen: String,
    pub last_seen: String,
    pub resolved: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SuspiciousRow {
    pub id: i64,
    pub server_id: String,
    pub ts: String,
    pub player: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub details: String,
    pub acknowledged: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerRow {
    pub name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub sessions: i64,
    pub total_minutes: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PublishRow {
    pub id: i64,
    pub launcher_id: String,
    pub ts: String,
    pub version: String,
    pub files: i64,
    pub changed: i64,
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Counts {
    pub bugs_open: i64,
    pub suspicious_open: i64,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS bugs (
  id INTEGER PRIMARY KEY,
  server_id TEXT NOT NULL,
  fingerprint TEXT NOT NULL,
  category TEXT NOT NULL,
  level TEXT NOT NULL,
  title TEXT NOT NULL,
  exception TEXT NOT NULL DEFAULT '',
  details TEXT NOT NULL DEFAULT '',
  count INTEGER NOT NULL DEFAULT 1,
  first_seen TEXT NOT NULL,
  last_seen TEXT NOT NULL,
  resolved INTEGER NOT NULL DEFAULT 0,
  UNIQUE(server_id, fingerprint)
);
CREATE TABLE IF NOT EXISTS suspicious (
  id INTEGER PRIMARY KEY,
  server_id TEXT NOT NULL,
  ts TEXT NOT NULL,
  player TEXT NOT NULL,
  category TEXT NOT NULL,
  severity TEXT NOT NULL,
  title TEXT NOT NULL,
  details TEXT NOT NULL DEFAULT '',
  acknowledged INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS suspicious_server ON suspicious(server_id, ts);
CREATE TABLE IF NOT EXISTS players (
  server_id TEXT NOT NULL,
  name TEXT NOT NULL,
  first_seen TEXT NOT NULL,
  last_seen TEXT NOT NULL,
  sessions INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(server_id, name)
);
CREATE TABLE IF NOT EXISTS sessions (
  id INTEGER PRIMARY KEY,
  server_id TEXT NOT NULL,
  player TEXT NOT NULL,
  joined TEXT NOT NULL,
  left TEXT
);
CREATE INDEX IF NOT EXISTS sessions_player ON sessions(server_id, player);
CREATE TABLE IF NOT EXISTS publishes (
  id INTEGER PRIMARY KEY,
  launcher_id TEXT NOT NULL,
  ts TEXT NOT NULL,
  version TEXT NOT NULL,
  files INTEGER NOT NULL,
  changed INTEGER NOT NULL,
  ok INTEGER NOT NULL,
  message TEXT NOT NULL
);
"#;

fn err(e: rusqlite::Error) -> String {
    format!("Historique : {e}")
}

impl History {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;").map_err(err)?;
        let h = Self { conn };
        h.conn.execute_batch(SCHEMA).map_err(err)?;
        Ok(h)
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(err)?;
        conn.execute_batch(SCHEMA).map_err(err)?;
        Ok(Self { conn })
    }

    pub fn record_bug(&self, server_id: &str, bug: &BugReport, ts: &str) -> Result<BugRow, String> {
        self.conn
            .execute(
                "INSERT INTO bugs(server_id, fingerprint, category, level, title, exception, details, count, first_seen, last_seen)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?8)
                 ON CONFLICT(server_id, fingerprint) DO UPDATE SET
                   count = count + 1, last_seen = excluded.last_seen, details = excluded.details, resolved = 0",
                params![server_id, bug.fingerprint, bug.category, bug.level, bug.title, bug.exception, bug.details, ts],
            )
            .map_err(err)?;
        self.conn
            .query_row(
                "SELECT id, server_id, category, level, title, exception, details, count, first_seen, last_seen, resolved
                 FROM bugs WHERE server_id = ?1 AND fingerprint = ?2",
                params![server_id, bug.fingerprint],
                map_bug,
            )
            .map_err(err)
    }

    pub fn list_bugs(&self, server_id: Option<&str>, include_resolved: bool, limit: usize) -> Result<Vec<BugRow>, String> {
        let sql = format!(
            "SELECT id, server_id, category, level, title, exception, details, count, first_seen, last_seen, resolved
             FROM bugs WHERE (?1 IS NULL OR server_id = ?1) AND (?2 = 1 OR resolved = 0)
             ORDER BY last_seen DESC LIMIT {limit}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(err)?;
        let rows = stmt.query_map(params![server_id, include_resolved as i64], map_bug).map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn set_bug_resolved(&self, id: i64, resolved: bool) -> Result<(), String> {
        self.conn.execute("UPDATE bugs SET resolved = ?2 WHERE id = ?1", params![id, resolved as i64]).map_err(err)?;
        Ok(())
    }

    pub fn delete_bug(&self, id: i64) -> Result<(), String> {
        self.conn.execute("DELETE FROM bugs WHERE id = ?1", params![id]).map_err(err)?;
        Ok(())
    }

    pub fn record_suspicious(&self, server_id: &str, ev: &SuspiciousEvent, ts: &str) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO suspicious(server_id, ts, player, category, severity, title, details) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![server_id, ts, ev.player, ev.category, ev.severity.as_str(), ev.title, ev.details],
            )
            .map_err(err)?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_suspicious(&self, server_id: Option<&str>, limit: usize) -> Result<Vec<SuspiciousRow>, String> {
        let sql = format!(
            "SELECT id, server_id, ts, player, category, severity, title, details, acknowledged
             FROM suspicious WHERE (?1 IS NULL OR server_id = ?1) ORDER BY ts DESC LIMIT {limit}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(err)?;
        let rows = stmt
            .query_map(params![server_id], |r| {
                Ok(SuspiciousRow {
                    id: r.get(0)?,
                    server_id: r.get(1)?,
                    ts: r.get(2)?,
                    player: r.get(3)?,
                    category: r.get(4)?,
                    severity: r.get(5)?,
                    title: r.get(6)?,
                    details: r.get(7)?,
                    acknowledged: r.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn ack_suspicious(&self, id: i64, ack: bool) -> Result<(), String> {
        self.conn.execute("UPDATE suspicious SET acknowledged = ?2 WHERE id = ?1", params![id, ack as i64]).map_err(err)?;
        Ok(())
    }

    pub fn delete_suspicious(&self, id: i64) -> Result<(), String> {
        self.conn.execute("DELETE FROM suspicious WHERE id = ?1", params![id]).map_err(err)?;
        Ok(())
    }

    /// Dernière alerte de minage pour (joueur, règle), pour ne pas la répéter à chaque scan.
    pub fn last_mining_alert(&self, server_id: &str, player: &str, rule: &str) -> Result<Option<String>, String> {
        self.conn
            .query_row(
                "SELECT ts FROM suspicious WHERE server_id = ?1 AND player = ?2 AND category = 'mining' AND title LIKE ?3
                 ORDER BY ts DESC LIMIT 1",
                params![server_id, player, format!("%{rule}%")],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(err)
    }

    pub fn player_joined(&self, server_id: &str, name: &str, ts: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO players(server_id, name, first_seen, last_seen, sessions) VALUES(?1, ?2, ?3, ?3, 1)
                 ON CONFLICT(server_id, name) DO UPDATE SET last_seen = excluded.last_seen, sessions = sessions + 1",
                params![server_id, name, ts],
            )
            .map_err(err)?;
        self.conn
            .execute("UPDATE sessions SET left = ?3 WHERE server_id = ?1 AND player = ?2 AND left IS NULL", params![server_id, name, ts])
            .map_err(err)?;
        self.conn
            .execute("INSERT INTO sessions(server_id, player, joined) VALUES(?1, ?2, ?3)", params![server_id, name, ts])
            .map_err(err)?;
        Ok(())
    }

    pub fn player_left(&self, server_id: &str, name: &str, ts: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE players SET last_seen = ?3 WHERE server_id = ?1 AND name = ?2", params![server_id, name, ts])
            .map_err(err)?;
        self.conn
            .execute("UPDATE sessions SET left = ?3 WHERE server_id = ?1 AND player = ?2 AND left IS NULL", params![server_id, name, ts])
            .map_err(err)?;
        Ok(())
    }

    /// Ferme toutes les sessions ouvertes d'un serveur (arrêt ou crash).
    pub fn close_sessions(&self, server_id: &str, ts: &str) -> Result<(), String> {
        self.conn
            .execute("UPDATE sessions SET left = ?2 WHERE server_id = ?1 AND left IS NULL", params![server_id, ts])
            .map_err(err)?;
        Ok(())
    }

    pub fn first_joins(&self, server_id: &str) -> Result<HashMap<String, DateTime<Local>>, String> {
        let mut stmt = self.conn.prepare("SELECT name, first_seen FROM players WHERE server_id = ?1").map_err(err)?;
        let rows = stmt
            .query_map(params![server_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(err)?;
        let mut map = HashMap::new();
        for row in rows.flatten() {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&row.1) {
                map.insert(row.0, dt.with_timezone(&Local));
            }
        }
        Ok(map)
    }

    pub fn list_players(&self, server_id: &str, limit: usize) -> Result<Vec<PlayerRow>, String> {
        let sql = format!(
            "SELECT p.name, p.first_seen, p.last_seen, p.sessions,
               COALESCE((SELECT SUM((julianday(COALESCE(s.left, datetime('now','localtime'))) - julianday(s.joined)) * 1440)
                         FROM sessions s WHERE s.server_id = p.server_id AND s.player = p.name), 0)
             FROM players p WHERE p.server_id = ?1 ORDER BY p.last_seen DESC LIMIT {limit}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(err)?;
        let rows = stmt
            .query_map(params![server_id], |r| {
                Ok(PlayerRow {
                    name: r.get(0)?,
                    first_seen: r.get(1)?,
                    last_seen: r.get(2)?,
                    sessions: r.get(3)?,
                    total_minutes: r.get::<_, f64>(4)?.round() as i64,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn record_publish(&self, launcher_id: &str, report: &PublishReport, ok: bool, ts: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO publishes(launcher_id, ts, version, files, changed, ok, message) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![launcher_id, ts, report.version, report.files_published as i64, report.files_changed as i64, ok as i64, report.message],
            )
            .map_err(err)?;
        Ok(())
    }

    pub fn list_publishes(&self, launcher_id: &str, limit: usize) -> Result<Vec<PublishRow>, String> {
        let sql = format!("SELECT id, launcher_id, ts, version, files, changed, ok, message FROM publishes WHERE launcher_id = ?1 ORDER BY ts DESC LIMIT {limit}");
        let mut stmt = self.conn.prepare(&sql).map_err(err)?;
        let rows = stmt
            .query_map(params![launcher_id], |r| {
                Ok(PublishRow {
                    id: r.get(0)?,
                    launcher_id: r.get(1)?,
                    ts: r.get(2)?,
                    version: r.get(3)?,
                    files: r.get(4)?,
                    changed: r.get(5)?,
                    ok: r.get::<_, i64>(6)? != 0,
                    message: r.get(7)?,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn counts(&self, server_id: &str) -> Result<Counts, String> {
        let bugs_open = self
            .conn
            .query_row("SELECT COUNT(*) FROM bugs WHERE server_id = ?1 AND resolved = 0", params![server_id], |r| r.get(0))
            .map_err(err)?;
        let suspicious_open = self
            .conn
            .query_row("SELECT COUNT(*) FROM suspicious WHERE server_id = ?1 AND acknowledged = 0", params![server_id], |r| r.get(0))
            .map_err(err)?;
        Ok(Counts { bugs_open, suspicious_open })
    }

    pub fn clear_server(&self, server_id: &str, bugs: bool, suspicious: bool) -> Result<(), String> {
        if bugs {
            self.conn.execute("DELETE FROM bugs WHERE server_id = ?1", params![server_id]).map_err(err)?;
        }
        if suspicious {
            self.conn.execute("DELETE FROM suspicious WHERE server_id = ?1", params![server_id]).map_err(err)?;
        }
        Ok(())
    }
}

fn map_bug(r: &rusqlite::Row) -> rusqlite::Result<BugRow> {
    Ok(BugRow {
        id: r.get(0)?,
        server_id: r.get(1)?,
        category: r.get(2)?,
        level: r.get(3)?,
        title: r.get(4)?,
        exception: r.get(5)?,
        details: r.get(6)?,
        count: r.get(7)?,
        first_seen: r.get(8)?,
        last_seen: r.get(9)?,
        resolved: r.get::<_, i64>(10)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suspicious::Severity;

    fn bug(fp: &str) -> BugReport {
        BugReport {
            category: "error".into(),
            level: "error".into(),
            title: "Boom".into(),
            exception: "NullPointerException".into(),
            details: "Boom\n\tat x".into(),
            fingerprint: fp.into(),
        }
    }

    #[test]
    fn bugs_are_grouped_and_counted() {
        let h = History::open_in_memory().unwrap();
        let a = h.record_bug("s1", &bug("fp1"), "2026-10-06T10:00:00+02:00").unwrap();
        assert_eq!(a.count, 1);
        let b = h.record_bug("s1", &bug("fp1"), "2026-10-06T11:00:00+02:00").unwrap();
        assert_eq!((b.id, b.count), (a.id, 2));
        h.record_bug("s2", &bug("fp1"), "2026-10-06T11:00:00+02:00").unwrap();
        assert_eq!(h.list_bugs(Some("s1"), true, 50).unwrap().len(), 1);
        assert_eq!(h.list_bugs(None, true, 50).unwrap().len(), 2);
        h.set_bug_resolved(a.id, true).unwrap();
        assert!(h.list_bugs(Some("s1"), false, 50).unwrap().is_empty());
        // Une nouvelle occurrence rouvre le bug.
        let c = h.record_bug("s1", &bug("fp1"), "2026-10-06T12:00:00+02:00").unwrap();
        assert!(!c.resolved);
        assert_eq!(h.counts("s1").unwrap().bugs_open, 1);
    }

    #[test]
    fn players_sessions_and_first_join() {
        let h = History::open_in_memory().unwrap();
        h.player_joined("s1", "Steve", "2026-10-06T10:00:00+02:00").unwrap();
        h.player_left("s1", "Steve", "2026-10-06T10:30:00+02:00").unwrap();
        h.player_joined("s1", "Steve", "2026-10-06T11:00:00+02:00").unwrap();
        h.player_left("s1", "Steve", "2026-10-06T11:15:00+02:00").unwrap();
        let players = h.list_players("s1", 10).unwrap();
        assert_eq!(players[0].sessions, 2);
        assert_eq!(players[0].total_minutes, 45);
        let first = h.first_joins("s1").unwrap();
        assert_eq!(first["Steve"], DateTime::parse_from_rfc3339("2026-10-06T10:00:00+02:00").unwrap());
    }

    #[test]
    fn suspicious_and_publishes() {
        let h = History::open_in_memory().unwrap();
        let ev = SuspiciousEvent {
            severity: Severity::High,
            player: "Steve".into(),
            category: "mining".into(),
            title: "Steve : Diamants 90/h".into(),
            details: String::new(),
        };
        let id = h.record_suspicious("s1", &ev, "2026-10-06T10:00:00+02:00").unwrap();
        assert_eq!(h.list_suspicious(Some("s1"), 10).unwrap()[0].severity, "high");
        assert!(h.last_mining_alert("s1", "Steve", "Diamants").unwrap().is_some());
        assert!(h.last_mining_alert("s1", "Alex", "Diamants").unwrap().is_none());
        h.ack_suspicious(id, true).unwrap();
        assert_eq!(h.counts("s1").unwrap().suspicious_open, 0);
        let report = PublishReport { version: "v1".into(), files_published: 3, files_changed: 1, committed: true, pushed: true, message: "ok".into() };
        h.record_publish("l1", &report, true, "2026-10-06T10:00:00+02:00").unwrap();
        assert_eq!(h.list_publishes("l1", 5).unwrap()[0].files, 3);
    }
}
