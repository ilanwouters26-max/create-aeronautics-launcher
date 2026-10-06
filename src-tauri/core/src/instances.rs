//! Détection des instances Minecraft côté client (CurseForge, launcher
//! officiel, Prism) et inspection d'un dossier choisi à la main.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DetectedInstance {
    /// "curseforge" | "official" | "prism" | "manual"
    pub source: String,
    pub name: String,
    pub dir: String,
    pub mc_version: String,
    pub loader: String,
    pub mods_count: usize,
}

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

pub fn user_profile_dir() -> Option<PathBuf> {
    env_path("USERPROFILE").or_else(|| env_path("HOME"))
}

pub fn appdata_dir() -> Option<PathBuf> {
    env_path("APPDATA").or_else(|| env_path("HOME").map(|h| h.join(".config")))
}

pub fn default_curseforge_root() -> Option<PathBuf> {
    user_profile_dir().map(|p| p.join("curseforge").join("minecraft").join("Instances"))
}

pub fn detect() -> Vec<DetectedInstance> {
    let mut out = Vec::new();
    if let Some(root) = default_curseforge_root() {
        out.extend(detect_curseforge(&root));
    }
    if let Some(appdata) = appdata_dir() {
        out.extend(detect_official(&appdata.join(".minecraft")));
        out.extend(detect_prism(&appdata.join("PrismLauncher").join("instances")));
    }
    out
}

pub fn count_mods(dir: &Path) -> usize {
    fs::read_dir(dir.join("mods"))
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_file())
                .filter(|e| e.file_name().to_str().map(crate::mods::is_mod_file).unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
}

pub fn detect_curseforge(root: &Path) -> Vec<DetectedInstance> {
    let Ok(rd) = fs::read_dir(root) else { return Vec::new() };
    let mut out: Vec<DetectedInstance> = rd
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let mut inst = inspect_dir(&e.path());
            inst.source = "curseforge".into();
            inst
        })
        .collect();
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out
}

/// Lit minecraftinstance.json (CurseForge) s'il existe, sinon décrit le dossier.
pub fn inspect_dir(dir: &Path) -> DetectedInstance {
    let mut inst = DetectedInstance {
        source: "manual".into(),
        name: dir.file_name().and_then(|n| n.to_str()).unwrap_or("instance").to_string(),
        dir: dir.to_string_lossy().to_string(),
        mods_count: count_mods(dir),
        ..Default::default()
    };
    if let Ok(text) = fs::read_to_string(dir.join("minecraftinstance.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(n) = v.get("name").and_then(|x| x.as_str()) {
                if !n.trim().is_empty() {
                    inst.name = n.to_string();
                }
            }
            inst.mc_version = v.get("gameVersion").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let loader_name = v.get("baseModLoader").and_then(|b| b.get("name")).and_then(|x| x.as_str()).unwrap_or("");
            inst.loader = loader_name.split('-').next().unwrap_or("").to_ascii_lowercase();
            if inst.mc_version.is_empty() {
                inst.mc_version = v
                    .get("baseModLoader")
                    .and_then(|b| b.get("minecraftVersion"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
            }
        }
    }
    inst
}

pub fn detect_official(minecraft_dir: &Path) -> Vec<DetectedInstance> {
    if !minecraft_dir.is_dir() {
        return Vec::new();
    }
    let mut out = vec![DetectedInstance {
        source: "official".into(),
        name: "Minecraft Launcher (.minecraft)".into(),
        dir: minecraft_dir.to_string_lossy().to_string(),
        mods_count: count_mods(minecraft_dir),
        ..Default::default()
    }];
    if let Ok(text) = fs::read_to_string(minecraft_dir.join("launcher_profiles.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(profiles) = v.get("profiles").and_then(|p| p.as_object()) {
                for (_, p) in profiles {
                    let Some(game_dir) = p.get("gameDir").and_then(|g| g.as_str()) else { continue };
                    let dir = PathBuf::from(game_dir);
                    if dir == minecraft_dir || !dir.is_dir() {
                        continue;
                    }
                    let name = p.get("name").and_then(|n| n.as_str()).unwrap_or("Profil").to_string();
                    let version = p.get("lastVersionId").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    out.push(DetectedInstance {
                        source: "official".into(),
                        name,
                        dir: dir.to_string_lossy().to_string(),
                        mc_version: version,
                        loader: String::new(),
                        mods_count: count_mods(&dir),
                    });
                }
            }
        }
    }
    out
}

pub fn detect_prism(root: &Path) -> Vec<DetectedInstance> {
    let Ok(rd) = fs::read_dir(root) else { return Vec::new() };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let path = e.path();
        if !path.is_dir() {
            continue;
        }
        let game_dir = if path.join(".minecraft").is_dir() {
            path.join(".minecraft")
        } else if path.join("minecraft").is_dir() {
            path.join("minecraft")
        } else {
            continue;
        };
        let mut name = path.file_name().and_then(|n| n.to_str()).unwrap_or("instance").to_string();
        if let Ok(cfg) = fs::read_to_string(path.join("instance.cfg")) {
            if let Some(n) = cfg.lines().find_map(|l| l.strip_prefix("name=")) {
                name = n.trim().to_string();
            }
        }
        let (mut mc_version, mut loader) = (String::new(), String::new());
        if let Ok(text) = fs::read_to_string(path.join("mmc-pack.json")) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                for c in v.get("components").and_then(|c| c.as_array()).cloned().unwrap_or_default() {
                    let uid = c.get("uid").and_then(|u| u.as_str()).unwrap_or("");
                    let ver = c.get("version").and_then(|u| u.as_str()).unwrap_or("").to_string();
                    match uid {
                        "net.minecraft" => mc_version = ver,
                        "net.fabricmc.fabric-loader" => loader = "fabric".into(),
                        "net.neoforged" => loader = "neoforge".into(),
                        "net.minecraftforge" => loader = "forge".into(),
                        "org.quiltmc.quilt-loader" => loader = "quilt".into(),
                        _ => {}
                    }
                }
            }
        }
        out.push(DetectedInstance {
            source: "prism".into(),
            name,
            dir: game_dir.to_string_lossy().to_string(),
            mc_version,
            loader,
            mods_count: count_mods(&game_dir),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_curseforge_layout() {
        let root = std::env::temp_dir().join(format!("panel-cf-{}", crate::config::new_id()));
        let inst = root.join("Create Aeronautics");
        fs::create_dir_all(inst.join("mods")).unwrap();
        fs::write(inst.join("mods/a.jar"), "").unwrap();
        fs::write(inst.join("mods/b.jar.disabled"), "").unwrap();
        fs::write(inst.join("mods/readme.txt"), "").unwrap();
        fs::write(
            inst.join("minecraftinstance.json"),
            r#"{"name":"Create Aeronautics","gameVersion":"1.21.1","baseModLoader":{"name":"neoforge-21.1.72","minecraftVersion":"1.21.1"}}"#,
        )
        .unwrap();
        let found = detect_curseforge(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source, "curseforge");
        assert_eq!(found[0].loader, "neoforge");
        assert_eq!(found[0].mc_version, "1.21.1");
        assert_eq!(found[0].mods_count, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detects_prism_and_official() {
        let root = std::env::temp_dir().join(format!("panel-prism-{}", crate::config::new_id()));
        let inst = root.join("instances/Mon pack");
        fs::create_dir_all(inst.join(".minecraft/mods")).unwrap();
        fs::write(inst.join("instance.cfg"), "InstanceType=OneSix\nname=Mon pack\n").unwrap();
        fs::write(
            inst.join("mmc-pack.json"),
            r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.15.0"}]}"#,
        )
        .unwrap();
        let found = detect_prism(&root.join("instances"));
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].name.as_str(), found[0].loader.as_str(), found[0].mc_version.as_str()), ("Mon pack", "fabric", "1.20.1"));

        let mc = root.join(".minecraft");
        fs::create_dir_all(mc.join("mods")).unwrap();
        let alt = root.join("alt");
        fs::create_dir_all(&alt).unwrap();
        fs::write(
            mc.join("launcher_profiles.json"),
            format!(r#"{{"profiles":{{"a":{{"name":"Fabric 1.21","gameDir":"{}","lastVersionId":"fabric-loader-0.16.5-1.21.1"}}}}}}"#, alt.to_string_lossy().replace('\\', "\\\\")),
        )
        .unwrap();
        let off = detect_official(&mc);
        assert_eq!(off.len(), 2);
        assert_eq!(off[1].name, "Fabric 1.21");
        let _ = fs::remove_dir_all(root);
    }
}
