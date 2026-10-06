//! Modèle de configuration de l'application, persisté en JSON.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppConfig {
    pub version: u32,
    pub settings: AppSettings,
    pub servers: Vec<ServerConfig>,
    pub instances: Vec<InstanceConfig>,
    pub launchers: Vec<LauncherConfig>,
    pub shutdown: Option<PlannedShutdown>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            settings: AppSettings::default(),
            servers: Vec::new(),
            instances: Vec::new(),
            launchers: Vec::new(),
            shutdown: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppSettings {
    /// Fermer la fenêtre la réduit dans la barre système au lieu de quitter.
    pub close_to_tray: bool,
    /// Démarrer caché dans la barre système (utile avec le lancement au démarrage de Windows).
    pub start_minimized: bool,
    /// Intervalle d'échantillonnage CPU/RAM quand l'onglet Ressources est visible.
    pub metrics_interval_visible_ms: u64,
    /// Intervalle quand rien ne regarde les métriques (0 = aucun échantillonnage).
    pub metrics_interval_hidden_ms: u64,
    /// Ping des serveurs distants / vérification d'état.
    pub ping_interval_s: u64,
    /// Fréquence d'analyse des fichiers world/stats (minutes).
    pub stats_scan_interval_min: u64,
    /// Nombre de lignes de console gardées en mémoire par serveur.
    pub log_buffer_lines: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_minimized: false,
            metrics_interval_visible_ms: 2000,
            metrics_interval_hidden_ms: 30_000,
            ping_interval_s: 30,
            stats_scan_interval_min: 5,
            log_buffer_lines: 2000,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub kind: ServerKind,
    /// "vanilla" | "fabric" | "forge" | "neoforge" | texte libre
    pub loader: String,
    pub mc_version: String,
    pub schedule: ScheduleConfig,
    pub detection: DetectionConfig,
    pub playit: PlayitConfig,
    pub notes: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            id: new_id(),
            name: "Nouveau serveur".into(),
            kind: ServerKind::default(),
            loader: String::new(),
            mc_version: String::new(),
            schedule: ScheduleConfig::default(),
            detection: DetectionConfig::default(),
            playit: PlayitConfig::default(),
            notes: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerKind {
    Local(LocalServer),
    Remote(RemoteServer),
}

impl Default for ServerKind {
    fn default() -> Self {
        ServerKind::Local(LocalServer::default())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LocalServer {
    /// Dossier du serveur (contient server.properties, mods/, world/).
    pub dir: String,
    /// Exécutable Java ("java" = celui du PATH).
    pub java_path: String,
    /// Arguments JVM (-Xmx, -Xms, flags GC...).
    pub jvm_args: String,
    pub launch: LaunchTarget,
    /// Arguments après le jar / fichier d'arguments (ex: nogui).
    pub extra_args: String,
    pub port: u16,
    pub auto_restart: bool,
    /// Délai max entre "stop" et le kill forcé.
    pub stop_timeout_s: u64,
    /// Annonce in-game avant un arrêt planifié (secondes, 0 = aucune).
    pub stop_warning_s: u64,
}

impl Default for LocalServer {
    fn default() -> Self {
        Self {
            dir: String::new(),
            java_path: "java".into(),
            jvm_args: "-Xms2G -Xmx6G -Dfile.encoding=UTF-8 -Dsun.stdout.encoding=UTF-8 -Dstdout.encoding=UTF-8"
                .into(),
            launch: LaunchTarget::default(),
            extra_args: "nogui".into(),
            port: 25565,
            auto_restart: false,
            stop_timeout_s: 90,
            stop_warning_s: 300,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum LaunchTarget {
    /// java -jar <path>
    Jar { path: String },
    /// java @<path> (Forge / NeoForge : win_args.txt)
    ArgsFile { path: String },
    /// Script .bat lancé via cmd (dernier recours : la console marche, pas le kill propre du java).
    Script { path: String },
}

impl Default for LaunchTarget {
    fn default() -> Self {
        LaunchTarget::Jar { path: "server.jar".into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RemoteServer {
    pub host: String,
    pub port: u16,
    pub rcon: RconConfig,
}

impl Default for RemoteServer {
    fn default() -> Self {
        Self { host: String::new(), port: 25565, rcon: RconConfig::default() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct RconConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub password: String,
}

impl Default for RconConfig {
    fn default() -> Self {
        Self { enabled: false, host: String::new(), port: 25575, password: String::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ScheduleConfig {
    pub enabled: bool,
    /// "HH:MM" heure locale, vide = pas d'ouverture automatique.
    pub open_time: String,
    /// "HH:MM" heure locale, vide = pas de fermeture automatique.
    pub close_time: String,
    /// Annonce in-game avant la fermeture (secondes).
    pub warning_s: u64,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self { enabled: false, open_time: "06:00".into(), close_time: "22:00".into(), warning_s: 300 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct DetectionConfig {
    pub enabled: bool,
    pub gamemode: bool,
    pub give: bool,
    pub op: bool,
    pub teleport: bool,
    pub other_admin: bool,
    pub early_advancements: bool,
    pub early_rules: Vec<EarlyAdvancementRule>,
    pub mining: bool,
    pub mining_rules: Vec<MiningRule>,
    /// Temps de jeu minimum avant d'évaluer un taux de minage (minutes).
    pub mining_min_playtime_min: u64,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gamemode: true,
            give: true,
            op: true,
            teleport: true,
            other_admin: false,
            early_advancements: true,
            early_rules: vec![
                EarlyAdvancementRule { title: "Diamonds!".into(), max_minutes: 20 },
                EarlyAdvancementRule { title: "Cover Me with Diamonds".into(), max_minutes: 60 },
                EarlyAdvancementRule { title: "Cover Me in Debris".into(), max_minutes: 180 },
            ],
            mining: true,
            mining_rules: vec![
                MiningRule {
                    label: "Diamants".into(),
                    blocks: vec!["minecraft:diamond_ore".into(), "minecraft:deepslate_diamond_ore".into()],
                    per_hour: 40.0,
                },
                MiningRule {
                    label: "Débris antiques".into(),
                    blocks: vec!["minecraft:ancient_debris".into()],
                    per_hour: 15.0,
                },
            ],
            mining_min_playtime_min: 30,
        }
    }
}

impl DetectionConfig {
    /// Préréglage pour un serveur où miner vite est le but du jeu (Mineral Contest).
    pub fn mining_event_preset() -> Self {
        Self { early_advancements: false, mining: false, ..Self::default() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EarlyAdvancementRule {
    /// Titre tel qu'il apparaît dans les logs ("Diamonds!").
    pub title: String,
    pub max_minutes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MiningRule {
    pub label: String,
    /// IDs de blocs sommés (variantes deepslate incluses).
    pub blocks: Vec<String>,
    /// Seuil d'alerte en blocs minés par heure de jeu.
    pub per_hour: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct PlayitConfig {
    pub enabled: bool,
    pub exe_path: String,
    pub args: String,
    /// Lancer l'agent avec le serveur et l'arrêter avec lui.
    pub with_server: bool,
}

impl Default for PlayitConfig {
    fn default() -> Self {
        Self { enabled: false, exe_path: String::new(), args: String::new(), with_server: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    /// Dossier contenant mods/ (dossier de l'instance CurseForge, .minecraft, etc.).
    pub dir: String,
    /// "curseforge" | "official" | "prism" | "manual"
    pub source: String,
    pub loader: String,
    pub mc_version: String,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            id: new_id(),
            name: String::new(),
            dir: String::new(),
            source: "manual".into(),
            loader: String::new(),
            mc_version: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LauncherConfig {
    pub id: String,
    pub name: String,
    /// Dossier du projet launcher (racine git).
    pub project_dir: String,
    /// Dossier des fichiers publiés, relatif au projet.
    pub release_dir: String,
    /// Dossier de build du launcher, relatif au projet (informatif).
    pub dist_dir: String,
    /// Chemin du manifest généré, relatif au projet.
    pub manifest_path: String,
    pub branch: String,
    /// Préfixe des URL de téléchargement écrites dans le manifest.
    pub base_url: String,
    /// Modèle du message de commit, {date} et {version} sont remplacés.
    pub commit_message: String,
    /// Chemins relatifs (dans release_dir) exclus de la publication.
    pub excluded: Vec<String>,
    /// Motifs de fichiers ignorés au scan (suffixes simples : ".disabled", ".bak").
    pub ignore_suffixes: Vec<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            id: new_id(),
            name: "Launcher".into(),
            project_dir: String::new(),
            release_dir: "release files".into(),
            dist_dir: "dist release".into(),
            manifest_path: "manifest.json".into(),
            branch: "main".into(),
            base_url: String::new(),
            commit_message: "Publication modpack {date}".into(),
            excluded: Vec::new(),
            ignore_suffixes: vec![".disabled".into(), ".bak".into(), ".tmp".into()],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct PlannedShutdown {
    /// Date/heure locale RFC3339.
    pub at: String,
    /// Arrêter proprement les serveurs gérés avant l'extinction.
    pub stop_servers: bool,
    /// Délai laissé par Windows pour annuler (secondes).
    pub grace_s: u64,
    /// Annonce in-game avant la séquence (secondes).
    pub warning_s: u64,
    pub created_at: String,
}

impl Default for PlannedShutdown {
    fn default() -> Self {
        Self { at: String::new(), stop_servers: true, grace_s: 60, warning_s: 300, created_at: String::new() }
    }
}

/// Identifiant court, unique dans la pratique (horloge + mélange xorshift).
pub fn new_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0) as u64;
    let mut x = nanos ^ 0x9E37_79B9_7F4A_7C15;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    format!("{:08x}{:04x}", (nanos & 0xFFFF_FFFF) as u32, (x & 0xFFFF) as u16)
}

pub fn load(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("Lecture config : {e}"))?;
    let mut cfg: AppConfig = serde_json::from_str(&text).map_err(|e| format!("Config invalide : {e}"))?;
    cfg.version = CONFIG_VERSION;
    Ok(cfg)
}

/// Écriture atomique : fichier temporaire puis renommage.
pub fn save(path: &Path, cfg: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Création dossier config : {e}"))?;
    }
    let text = serde_json::to_string_pretty(cfg).map_err(|e| format!("Sérialisation config : {e}"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| format!("Écriture config : {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("Remplacement config : {e}"))?;
    Ok(())
}

impl AppConfig {
    pub fn server(&self, id: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.id == id)
    }
    pub fn server_mut(&mut self, id: &str) -> Option<&mut ServerConfig> {
        self.servers.iter_mut().find(|s| s.id == id)
    }
    pub fn launcher(&self, id: &str) -> Option<&LauncherConfig> {
        self.launchers.iter().find(|l| l.id == id)
    }
    pub fn servers_by_id(&self) -> HashMap<String, &ServerConfig> {
        self.servers.iter().map(|s| (s.id.clone(), s)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let mut cfg = AppConfig::default();
        let mut s = ServerConfig { name: "Zevent".into(), ..Default::default() };
        s.kind = ServerKind::Local(LocalServer { dir: "C:\\TelekGamer\\Zevent".into(), ..Default::default() });
        s.detection = DetectionConfig::mining_event_preset();
        cfg.servers.push(s);
        cfg.servers.push(ServerConfig {
            name: "Serv Adrien".into(),
            kind: ServerKind::Remote(RemoteServer { host: "game4.onpowered.net".into(), port: 25582, ..Default::default() }),
            ..Default::default()
        });
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"type\":\"remote\""));
        assert!(json.contains("\"mode\":\"jar\""));
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.servers.len(), 2);
        assert!(!back.servers[0].detection.mining);
        match &back.servers[1].kind {
            ServerKind::Remote(r) => assert_eq!(r.port, 25582),
            _ => panic!("remote attendu"),
        }
    }

    #[test]
    fn missing_fields_take_defaults() {
        let cfg: AppConfig = serde_json::from_str(r#"{"servers":[{"name":"x"}]}"#).unwrap();
        assert_eq!(cfg.servers[0].name, "x");
        assert!(!cfg.servers[0].id.is_empty());
        assert_eq!(cfg.settings.log_buffer_lines, 2000);
    }

    #[test]
    fn ids_differ() {
        assert_ne!(new_id(), new_id());
    }

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join(format!("panel-core-test-{}", new_id()));
        let path = dir.join("config.json");
        let cfg = AppConfig::default();
        save(&path, &cfg).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.version, CONFIG_VERSION);
        let _ = fs::remove_dir_all(dir);
    }
}
