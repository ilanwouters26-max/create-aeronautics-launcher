//! Lecture des mods d'un dossier (métadonnées dans le jar), activation,
//! suppression, import, doublons et comparaison entre deux dossiers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ModInfo {
    pub file_name: String,
    pub path: String,
    pub id: String,
    pub name: String,
    pub version: String,
    /// "fabric" | "forge" | "neoforge" | "quilt" | "" ; plusieurs séparés par "+"
    pub loader: String,
    /// "client" | "server" | "both" | ""
    pub environment: String,
    pub description: String,
    pub size: u64,
    pub enabled: bool,
    pub modified: i64,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct JarMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub loader: String,
    pub environment: String,
    pub description: String,
}

pub fn is_mod_file(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".jar") || n.ends_with(&format!(".jar{DISABLED_SUFFIX}"))
}

pub fn scan_mods(dir: &Path) -> Vec<ModInfo> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<ModInfo> = rd
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let file_name = e.file_name().to_str()?.to_string();
            if !is_mod_file(&file_name) {
                return None;
            }
            Some(describe(&e.path(), &file_name))
        })
        .collect();
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out
}

fn describe(path: &Path, file_name: &str) -> ModInfo {
    let md = fs::metadata(path).ok();
    let size = md.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = md
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let enabled = !file_name.to_ascii_lowercase().ends_with(DISABLED_SUFFIX);
    let meta = read_jar_meta(path).unwrap_or_default();
    let stem = file_name
        .trim_end_matches(DISABLED_SUFFIX)
        .trim_end_matches(".jar")
        .trim_end_matches(".JAR")
        .to_string();
    ModInfo {
        file_name: file_name.to_string(),
        path: path.to_string_lossy().to_string(),
        id: meta.id,
        name: if meta.name.is_empty() { stem } else { meta.name },
        version: meta.version,
        loader: meta.loader,
        environment: meta.environment,
        description: meta.description,
        size,
        enabled,
        modified,
    }
}

fn read_entry(archive: &mut zip::ZipArchive<File>, name: &str) -> Option<String> {
    let mut f = archive.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    Some(s)
}

pub fn read_jar_meta(path: &Path) -> Option<JarMeta> {
    let file = File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut meta = JarMeta::default();
    let mut loaders = Vec::new();

    if let Some(text) = read_entry(&mut archive, "fabric.mod.json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            loaders.push("fabric");
            fill_from_fabric(&mut meta, &v);
        }
    }
    if let Some(text) = read_entry(&mut archive, "quilt.mod.json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            loaders.push("quilt");
            if meta.id.is_empty() {
                let ql = v.get("quilt_loader").cloned().unwrap_or_default();
                meta.id = str_of(ql.get("id"));
                meta.version = str_of(ql.get("version"));
                meta.name = str_of(ql.get("metadata").and_then(|m| m.get("name")));
                meta.description = str_of(ql.get("metadata").and_then(|m| m.get("description")));
                meta.environment = env_of(str_of(v.get("minecraft").and_then(|m| m.get("environment"))).as_str());
            }
        }
    }
    let manifest_version = read_entry(&mut archive, "META-INF/MANIFEST.MF").and_then(|m| {
        m.lines()
            .find_map(|l| l.strip_prefix("Implementation-Version:").map(|v| v.trim().to_string()))
    });
    for (entry, loader) in [("META-INF/neoforge.mods.toml", "neoforge"), ("META-INF/mods.toml", "forge")] {
        if let Some(text) = read_entry(&mut archive, entry) {
            if let Ok(v) = toml::from_str::<toml::Value>(&text) {
                loaders.push(loader);
                if meta.id.is_empty() {
                    fill_from_toml(&mut meta, &v, manifest_version.as_deref());
                }
            }
        }
    }
    if loaders.is_empty() {
        return None;
    }
    meta.loader = loaders.join("+");
    Some(meta)
}

fn str_of(v: Option<&serde_json::Value>) -> String {
    v.and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn env_of(s: &str) -> String {
    match s {
        "client" => "client".into(),
        "server" => "server".into(),
        "*" | "both" => "both".into(),
        _ => String::new(),
    }
}

fn fill_from_fabric(meta: &mut JarMeta, v: &serde_json::Value) {
    meta.id = str_of(v.get("id"));
    meta.version = str_of(v.get("version"));
    meta.name = str_of(v.get("name"));
    meta.description = str_of(v.get("description"));
    meta.environment = env_of(str_of(v.get("environment")).as_str());
}

fn fill_from_toml(meta: &mut JarMeta, v: &toml::Value, manifest_version: Option<&str>) {
    let Some(first) = v.get("mods").and_then(|m| m.as_array()).and_then(|a| a.first()) else { return };
    let get = |k: &str| first.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    meta.id = get("modId");
    meta.name = get("displayName");
    meta.description = get("description").lines().next().unwrap_or("").trim().to_string();
    let mut version = get("version");
    if version.contains("${") {
        version = manifest_version.map(String::from).unwrap_or_default();
    }
    meta.version = version;
}

/// Renomme x.jar ⇄ x.jar.disabled. Retourne le nouveau chemin.
pub fn set_enabled(path: &Path, enabled: bool) -> Result<PathBuf, String> {
    let name = path.file_name().and_then(|n| n.to_str()).ok_or("Nom de fichier invalide")?.to_string();
    let currently = !name.to_ascii_lowercase().ends_with(DISABLED_SUFFIX);
    if currently == enabled {
        return Ok(path.to_path_buf());
    }
    let new_name = if enabled {
        name[..name.len() - DISABLED_SUFFIX.len()].to_string()
    } else {
        format!("{name}{DISABLED_SUFFIX}")
    };
    let target = path.with_file_name(new_name);
    if target.exists() {
        return Err(format!("{} existe déjà", target.display()));
    }
    fs::rename(path, &target).map_err(|e| format!("Renommage : {e}"))?;
    Ok(target)
}

pub fn delete_mod(path: &Path) -> Result<(), String> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !is_mod_file(name) {
        return Err("Seuls les fichiers .jar sont supprimés par ici".into());
    }
    fs::remove_file(path).map_err(|e| format!("Suppression : {e}"))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImportResult {
    pub file: String,
    pub ok: bool,
    pub message: String,
}

/// Copie des jars dans le dossier mods (écrase un fichier de même nom).
pub fn import_mods(dir: &Path, files: &[PathBuf]) -> Vec<ImportResult> {
    let mut out = Vec::new();
    if let Err(e) = fs::create_dir_all(dir) {
        return vec![ImportResult { file: dir.to_string_lossy().to_string(), ok: false, message: format!("Dossier mods : {e}") }];
    }
    for f in files {
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if !is_mod_file(&name) {
            out.push(ImportResult { file: name, ok: false, message: "Pas un .jar, ignoré".into() });
            continue;
        }
        let target = dir.join(&name);
        let result = match fs::copy(f, &target) {
            Ok(_) => ImportResult { file: name, ok: true, message: "Copié".into() },
            Err(e) => ImportResult { file: name, ok: false, message: format!("Copie : {e}") },
        };
        out.push(result);
    }
    out
}

pub fn mod_key(m: &ModInfo) -> String {
    if !m.id.is_empty() {
        m.id.to_ascii_lowercase()
    } else {
        m.file_name.trim_end_matches(DISABLED_SUFFIX).to_ascii_lowercase()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DuplicateGroup {
    pub key: String,
    pub files: Vec<String>,
}

pub fn duplicates(mods: &[ModInfo]) -> Vec<DuplicateGroup> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for m in mods {
        groups.entry(mod_key(m)).or_default().push(m.file_name.clone());
    }
    let mut out: Vec<DuplicateGroup> = groups
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(key, files)| DuplicateGroup { key, files })
        .collect();
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ModDiff {
    pub only_a: Vec<ModInfo>,
    pub only_b: Vec<ModInfo>,
    pub version_mismatch: Vec<(ModInfo, ModInfo)>,
    pub same: usize,
}

pub fn diff(a: &[ModInfo], b: &[ModInfo]) -> ModDiff {
    let map_b: HashMap<String, &ModInfo> = b.iter().map(|m| (mod_key(m), m)).collect();
    let map_a: HashMap<String, &ModInfo> = a.iter().map(|m| (mod_key(m), m)).collect();
    let mut d = ModDiff::default();
    for m in a {
        match map_b.get(&mod_key(m)) {
            None => d.only_a.push(m.clone()),
            Some(other) => {
                if m.version != other.version || m.file_name != other.file_name {
                    d.version_mismatch.push((m.clone(), (*other).clone()));
                } else {
                    d.same += 1;
                }
            }
        }
    }
    for m in b {
        if !map_a.contains_key(&mod_key(m)) {
            d.only_b.push(m.clone());
        }
    }
    d
}

#[cfg(test)]
pub(crate) mod testutil {
    use std::io::Write;
    use std::path::Path;

    /// Fabrique un jar minimal contenant les entrées données.
    pub fn make_jar(path: &Path, entries: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::make_jar;
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("panel-mods-{}", crate::config::new_id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_fabric_and_neoforge_metadata() {
        let dir = tmp();
        make_jar(
            &dir.join("sodium-0.6.jar"),
            &[("fabric.mod.json", r#"{"id":"sodium","version":"0.6.0","name":"Sodium","environment":"client","description":"Fast"}"#)],
        );
        make_jar(
            &dir.join("create-6.0.jar"),
            &[
                ("META-INF/neoforge.mods.toml", "modLoader=\"javafml\"\n[[mods]]\nmodId=\"create\"\nversion=\"${file.jarVersion}\"\ndisplayName=\"Create\"\ndescription='''Big\nmulti'''\n"),
                ("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\nImplementation-Version: 6.0.4\n"),
            ],
        );
        make_jar(&dir.join("old.jar.disabled"), &[("fabric.mod.json", r#"{"id":"old","version":"1","name":"Old"}"#)]);
        fs::write(dir.join("readme.txt"), "x").unwrap();
        let mods = scan_mods(&dir);
        assert_eq!(mods.len(), 3);
        let sodium = mods.iter().find(|m| m.id == "sodium").unwrap();
        assert_eq!((sodium.loader.as_str(), sodium.environment.as_str(), sodium.version.as_str()), ("fabric", "client", "0.6.0"));
        let create = mods.iter().find(|m| m.id == "create").unwrap();
        assert_eq!((create.loader.as_str(), create.version.as_str(), create.name.as_str()), ("neoforge", "6.0.4", "Create"));
        assert_eq!(create.description, "Big");
        let old = mods.iter().find(|m| m.id == "old").unwrap();
        assert!(!old.enabled);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn enable_disable_delete_import() {
        let dir = tmp();
        let jar = dir.join("a.jar");
        make_jar(&jar, &[("fabric.mod.json", r#"{"id":"a","version":"1"}"#)]);
        let disabled = set_enabled(&jar, false).unwrap();
        assert!(disabled.to_string_lossy().ends_with("a.jar.disabled"));
        assert!(!scan_mods(&dir)[0].enabled);
        let back = set_enabled(&disabled, true).unwrap();
        assert_eq!(back, jar);
        let src = tmp();
        let incoming = src.join("b.jar");
        make_jar(&incoming, &[("fabric.mod.json", r#"{"id":"b","version":"2"}"#)]);
        fs::write(src.join("notes.txt"), "x").unwrap();
        let res = import_mods(&dir, &[incoming.clone(), src.join("notes.txt")]);
        assert!(res[0].ok);
        assert!(!res[1].ok);
        assert_eq!(scan_mods(&dir).len(), 2);
        delete_mod(&dir.join("b.jar")).unwrap();
        assert!(delete_mod(&src.join("notes.txt")).is_err());
        assert_eq!(scan_mods(&dir).len(), 1);
        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(src);
    }

    #[test]
    fn duplicates_and_diff() {
        let mk = |id: &str, ver: &str, file: &str| ModInfo {
            id: id.into(),
            version: ver.into(),
            file_name: file.into(),
            name: id.into(),
            enabled: true,
            ..Default::default()
        };
        let a = vec![mk("sodium", "1", "sodium-1.jar"), mk("sodium", "2", "sodium-2.jar"), mk("create", "6", "create.jar")];
        let dup = duplicates(&a);
        assert_eq!(dup.len(), 1);
        assert_eq!(dup[0].files.len(), 2);
        let b = vec![mk("create", "7", "create.jar"), mk("jei", "1", "jei.jar")];
        let d = diff(&a, &b);
        assert_eq!(d.only_a.len(), 2);
        assert_eq!(d.only_b.len(), 1);
        assert_eq!(d.version_mismatch.len(), 1);
        assert_eq!(d.same, 0);
    }
}
