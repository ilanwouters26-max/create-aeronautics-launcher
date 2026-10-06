//! Cœur métier du panneau, sans dépendance à Tauri : testable en ligne de commande.

pub mod bugs;
pub mod config;
pub mod history;
pub mod instances;
pub mod logs;
pub mod mods;
pub mod ping;
pub mod process;
pub mod properties;
pub mod publish;
pub mod rcon;
pub mod schedule;
pub mod stats;
pub mod suspicious;
pub mod sysmon;
pub mod system;

pub fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
