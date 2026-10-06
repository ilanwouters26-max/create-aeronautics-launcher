//! Planification : ouverture/fermeture quotidienne des serveurs et extinction
//! du PC. Logique pure, l'application applique les actions retournées.

use crate::config::{PlannedShutdown, ScheduleConfig};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    NaiveTime::parse_from_str(s, "%H:%M").ok().or_else(|| NaiveTime::parse_from_str(s, "%H:%M:%S").ok())
}

/// Vrai si `now` est dans la plage d'ouverture (gère le passage de minuit).
pub fn in_window(now: NaiveTime, open: Option<NaiveTime>, close: Option<NaiveTime>) -> bool {
    match (open, close) {
        (None, None) => false,
        (Some(o), None) => now >= o,
        (None, Some(c)) => now < c,
        (Some(o), Some(c)) => {
            if o < c {
                now >= o && now < c
            } else {
                now >= o || now < c
            }
        }
    }
}

fn seconds_until(now: NaiveTime, target: NaiveTime) -> i64 {
    let n = now.num_seconds_from_midnight() as i64;
    let t = target.num_seconds_from_midnight() as i64;
    (t - n).rem_euclid(86_400)
}

#[derive(Default)]
pub struct ScheduleState {
    was_in_window: HashMap<String, bool>,
    warned: HashMap<String, NaiveDate>,
    opened: HashMap<String, NaiveDate>,
    shutdown_warned: bool,
}

impl ScheduleState {
    pub fn reset_shutdown(&mut self) {
        self.shutdown_warned = false;
    }
}

pub struct ServerSnapshot<'a> {
    pub id: &'a str,
    pub schedule: &'a ScheduleConfig,
    pub running: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DueAction {
    Open { server_id: String },
    WarnClose { server_id: String, seconds: u64 },
    Close { server_id: String },
    ShutdownWarn { seconds: u64 },
    ShutdownNow,
}

pub fn due_actions(now: DateTime<Local>, servers: &[ServerSnapshot], state: &mut ScheduleState) -> Vec<DueAction> {
    let mut actions = Vec::new();
    let today = now.date_naive();
    let t = now.time();
    for s in servers {
        let id = s.id.to_string();
        if !s.schedule.enabled {
            state.was_in_window.remove(&id);
            continue;
        }
        let open = parse_hhmm(&s.schedule.open_time);
        let close = parse_hhmm(&s.schedule.close_time);
        if open.is_none() && close.is_none() {
            continue;
        }
        let inw = in_window(t, open, close);
        let prev = state.was_in_window.get(&id).copied();
        if inw && !s.running && prev != Some(true) && open.is_some() && state.opened.get(&id) != Some(&today) {
            state.opened.insert(id.clone(), today);
            actions.push(DueAction::Open { server_id: id.clone() });
        }
        if let Some(c) = close {
            if inw && s.running && s.schedule.warning_s > 0 {
                let until = seconds_until(t, c);
                if until > 0 && until <= s.schedule.warning_s as i64 && state.warned.get(&id) != Some(&today) {
                    state.warned.insert(id.clone(), today);
                    actions.push(DueAction::WarnClose { server_id: id.clone(), seconds: until as u64 });
                }
            }
            if prev == Some(true) && !inw && s.running {
                actions.push(DueAction::Close { server_id: id.clone() });
            }
        }
        state.was_in_window.insert(id, inw);
    }
    actions
}

pub fn parse_plan_time(plan: &PlannedShutdown) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(&plan.at).ok().map(|d| d.with_timezone(&Local))
}

/// Un plan dont l'heure est passée depuis longtemps (app fermée entre-temps) est abandonné.
pub fn shutdown_expired(now: DateTime<Local>, plan: &PlannedShutdown) -> bool {
    match parse_plan_time(plan) {
        Some(at) => now - at > Duration::minutes(10),
        None => true,
    }
}

pub fn shutdown_due(now: DateTime<Local>, plan: &PlannedShutdown, state: &mut ScheduleState) -> Option<DueAction> {
    let at = parse_plan_time(plan)?;
    let until = (at - now).num_seconds();
    if until <= 0 {
        return Some(DueAction::ShutdownNow);
    }
    if until <= plan.warning_s as i64 && !state.shutdown_warned {
        state.shutdown_warned = true;
        return Some(DueAction::ShutdownWarn { seconds: until as u64 });
    }
    None
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NextEvent {
    /// "open" | "close"
    pub kind: String,
    pub at: String,
    pub in_seconds: i64,
}

/// Prochaine ouverture ou fermeture planifiée, pour l'affichage.
pub fn next_event(now: DateTime<Local>, schedule: &ScheduleConfig) -> Option<NextEvent> {
    if !schedule.enabled {
        return None;
    }
    let mut best: Option<NextEvent> = None;
    for (kind, time) in [("open", parse_hhmm(&schedule.open_time)), ("close", parse_hhmm(&schedule.close_time))] {
        let Some(tm) = time else { continue };
        let mut secs = seconds_until(now.time(), tm);
        if secs == 0 {
            secs = 86_400;
        }
        let at = now + Duration::seconds(secs);
        let candidate = NextEvent { kind: kind.to_string(), at: at.to_rfc3339(), in_seconds: secs };
        if best.as_ref().map(|b| secs < b.in_seconds).unwrap_or(true) {
            best = Some(candidate);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 10, 6, h, m, 0).unwrap()
    }

    fn sched(open: &str, close: &str) -> ScheduleConfig {
        ScheduleConfig { enabled: true, open_time: open.into(), close_time: close.into(), warning_s: 300 }
    }

    #[test]
    fn window_handles_midnight() {
        let t = |h| NaiveTime::from_hms_opt(h, 0, 0).unwrap();
        assert!(in_window(t(12), Some(t(6)), Some(t(22))));
        assert!(!in_window(t(23), Some(t(6)), Some(t(22))));
        assert!(in_window(t(23), Some(t(22)), Some(t(6))));
        assert!(in_window(t(3), Some(t(22)), Some(t(6))));
        assert!(!in_window(t(12), Some(t(22)), Some(t(6))));
        assert!(!in_window(t(12), None, None));
    }

    #[test]
    fn opens_on_catch_up_then_warns_then_closes() {
        let s = sched("06:00", "22:00");
        let mut state = ScheduleState::default();
        let snap = |running| vec![ServerSnapshot { id: "a", schedule: &s, running }];
        // App lancée à 07:00, serveur arrêté : rattrapage.
        assert_eq!(due_actions(at(7, 0), &snap(false), &mut state), vec![DueAction::Open { server_id: "a".into() }]);
        // Tick suivant, toujours arrêté (en cours de démarrage) : pas de doublon.
        assert!(due_actions(at(7, 1), &snap(false), &mut state).is_empty());
        // Arrêt manuel en journée : pas de relance.
        assert!(due_actions(at(12, 0), &snap(false), &mut state).is_empty());
        // Avertissement 5 min avant la fermeture, une seule fois.
        assert_eq!(
            due_actions(at(21, 56), &snap(true), &mut state),
            vec![DueAction::WarnClose { server_id: "a".into(), seconds: 240 }]
        );
        assert!(due_actions(at(21, 57), &snap(true), &mut state).is_empty());
        // Sortie de plage : fermeture.
        assert_eq!(due_actions(at(22, 0), &snap(true), &mut state), vec![DueAction::Close { server_id: "a".into() }]);
        // Lancé à la main la nuit : on ne le ferme pas.
        assert!(due_actions(at(23, 0), &snap(true), &mut state).is_empty());
    }

    #[test]
    fn disabled_schedule_does_nothing() {
        let s = ScheduleConfig { enabled: false, ..sched("06:00", "22:00") };
        let mut state = ScheduleState::default();
        assert!(due_actions(at(7, 0), &[ServerSnapshot { id: "a", schedule: &s, running: false }], &mut state).is_empty());
    }

    #[test]
    fn shutdown_warns_then_fires() {
        let plan = PlannedShutdown { at: at(23, 0).to_rfc3339(), warning_s: 300, ..Default::default() };
        let mut state = ScheduleState::default();
        assert_eq!(shutdown_due(at(22, 0), &plan, &mut state), None);
        assert_eq!(shutdown_due(at(22, 56), &plan, &mut state), Some(DueAction::ShutdownWarn { seconds: 240 }));
        assert_eq!(shutdown_due(at(22, 57), &plan, &mut state), None);
        assert_eq!(shutdown_due(at(23, 0), &plan, &mut state), Some(DueAction::ShutdownNow));
        assert!(!shutdown_expired(at(23, 5), &plan));
        assert!(shutdown_expired(at(23, 30), &plan));
    }

    #[test]
    fn next_event_picks_soonest() {
        let s = sched("06:00", "22:00");
        let n = next_event(at(12, 0), &s).unwrap();
        assert_eq!(n.kind, "close");
        assert_eq!(n.in_seconds, 10 * 3600);
        let n = next_event(at(23, 0), &s).unwrap();
        assert_eq!(n.kind, "open");
        assert_eq!(n.in_seconds, 7 * 3600);
    }
}
