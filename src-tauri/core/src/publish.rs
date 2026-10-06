//! Publication d'un modpack vers GitHub : scan du dossier de release,
//! manifest, puis git add / commit / push avec sortie en direct.

use crate::config::LauncherConfig;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReleaseFile {
    /// Chemin relatif au dossier de release, séparateurs "/".
    pub rel_path: String,
    pub name: String,
    pub size: u64,
    pub sha1: String,
    pub modified: i64,
}

/// Cache des empreintes (chemin → taille, date, sha1) pour ne pas relire les jars à chaque scan.
#[derive(Default)]
pub struct ShaCache {
    map: HashMap<String, (u64, i64, String)>,
}

pub fn sha1_file(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| format!("{} : {e}", path.display()))?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn scan_release_dir(dir: &Path, ignore_suffixes: &[String], cache: &mut ShaCache) -> Result<Vec<ReleaseFile>, String> {
    if !dir.is_dir() {
        return Err(format!("Dossier de release introuvable : {}", dir.display()));
    }
    let mut files = Vec::new();
    walk(dir, dir, ignore_suffixes, cache, &mut files)?;
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(files)
}

fn walk(root: &Path, dir: &Path, ignore: &[String], cache: &mut ShaCache, out: &mut Vec<ReleaseFile>) -> Result<(), String> {
    let rd = fs::read_dir(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, ignore, cache, out)?;
            continue;
        }
        if ignore.iter().any(|s| !s.is_empty() && name.to_ascii_lowercase().ends_with(&s.to_ascii_lowercase())) {
            continue;
        }
        let md = entry.metadata().map_err(|e| e.to_string())?;
        let size = md.len();
        let modified = md.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
        let rel_path = path.strip_prefix(root).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        let sha1 = match cache.map.get(&rel_path) {
            Some((s, m, sha)) if *s == size && *m == modified => sha.clone(),
            _ => {
                let sha = sha1_file(&path)?;
                cache.map.insert(rel_path.clone(), (size, modified, sha.clone()));
                sha
            }
        };
        out.push(ReleaseFile { rel_path, name, size, sha1, modified });
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ManifestEntry {
    pub path: String,
    pub size: u64,
    pub sha1: String,
    pub url: String,
}

/// Format par défaut du manifest. À aligner sur celui que lisent les launchers
/// Electron existants : seul `render`/`build_manifest` change.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub version: String,
    pub generated_at: String,
    pub base_url: String,
    pub files: Vec<ManifestEntry>,
}

pub fn encode_url_path(rel: &str) -> String {
    let mut out = String::with_capacity(rel.len());
    for b in rel.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn build_manifest(files: &[ReleaseFile], excluded: &HashSet<String>, base_url: &str, version: &str) -> Manifest {
    let base = base_url.trim().trim_end_matches('/').replace(' ', "%20");
    let base = base.as_str();
    Manifest {
        version: version.to_string(),
        generated_at: chrono::Local::now().to_rfc3339(),
        base_url: base.to_string(),
        files: files
            .iter()
            .filter(|f| !excluded.contains(&f.rel_path))
            .map(|f| ManifestEntry {
                path: f.rel_path.clone(),
                size: f.size,
                sha1: f.sha1.clone(),
                url: if base.is_empty() { String::new() } else { format!("{base}/{}", encode_url_path(&f.rel_path)) },
            })
            .collect(),
    }
}

pub fn read_manifest(path: &Path) -> Option<Manifest> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_manifest(path: &Path, m: &Manifest) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| format!("Écriture manifest : {e}"))
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    New,
    Changed,
    Unchanged,
    Removed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileDiff {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub sha1: String,
    pub status: FileStatus,
}

pub fn compare(previous: Option<&Manifest>, current: &[ReleaseFile]) -> Vec<FileDiff> {
    let prev: HashMap<&str, &ManifestEntry> =
        previous.map(|m| m.files.iter().map(|e| (e.path.as_str(), e)).collect()).unwrap_or_default();
    let mut out: Vec<FileDiff> = current
        .iter()
        .map(|f| FileDiff {
            path: f.rel_path.clone(),
            name: f.name.clone(),
            size: f.size,
            sha1: f.sha1.clone(),
            status: match prev.get(f.rel_path.as_str()) {
                None => FileStatus::New,
                Some(e) if e.sha1 == f.sha1 => FileStatus::Unchanged,
                Some(_) => FileStatus::Changed,
            },
        })
        .collect();
    let now: HashSet<&str> = current.iter().map(|f| f.rel_path.as_str()).collect();
    for (path, e) in prev {
        if !now.contains(path) {
            out.push(FileDiff {
                path: path.to_string(),
                name: path.rsplit('/').next().unwrap_or(path).to_string(),
                size: e.size,
                sha1: e.sha1.clone(),
                status: FileStatus::Removed,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn git_command(repo: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).env("GIT_TERMINAL_PROMPT", "0").env("LC_ALL", "C");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Lance git et transmet chaque ligne (stdout et stderr) au fur et à mesure.
pub fn run_git(repo: &Path, args: &[&str], on_line: &dyn Fn(String)) -> Result<i32, String> {
    let mut child = git_command(repo)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git introuvable ou injoignable : {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let mut handles = Vec::new();
    for reader in [stdout.map(|s| Box::new(s) as Box<dyn Read + Send>), stderr.map(|s| Box::new(s) as Box<dyn Read + Send>)]
        .into_iter()
        .flatten()
    {
        let tx = tx.clone();
        handles.push(std::thread::spawn(move || {
            let mut r = BufReader::new(reader);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match r.read_until(b'\n', &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        // git affiche la progression avec des retours chariot
                        for part in String::from_utf8_lossy(&buf).split('\r') {
                            let p = part.trim_end_matches('\n');
                            if !p.trim().is_empty() {
                                let _ = tx.send(p.to_string());
                            }
                        }
                    }
                }
            }
        }));
    }
    drop(tx);
    for line in rx {
        on_line(line);
    }
    for h in handles {
        let _ = h.join();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(-1))
}

fn git_output(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = git_command(repo).args(args).stdin(Stdio::null()).output().map_err(|e| format!("git : {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct GitInfo {
    pub available: bool,
    pub is_repo: bool,
    pub branch: String,
    pub last_commit: String,
    pub remote: String,
    pub dirty_count: usize,
    pub error: String,
}

pub fn git_info(repo: &Path) -> GitInfo {
    let mut info = GitInfo::default();
    if !repo.is_dir() {
        info.error = "Dossier du projet introuvable".into();
        return info;
    }
    match git_output(repo, &["rev-parse", "--is-inside-work-tree"]) {
        Ok(v) => {
            info.available = true;
            info.is_repo = v == "true";
        }
        Err(e) => {
            info.available = !e.is_empty() || Command::new("git").arg("--version").output().is_ok();
            info.error = if info.available { "Pas un dépôt git".into() } else { "git n'est pas installé ou pas dans le PATH".into() };
            return info;
        }
    }
    info.branch = git_output(repo, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    info.last_commit = git_output(repo, &["log", "-1", "--format=%h %s (%cr)"]).unwrap_or_default();
    info.remote = git_output(repo, &["remote", "get-url", "origin"]).unwrap_or_default();
    info.dirty_count = git_output(repo, &["status", "--porcelain"]).map(|s| s.lines().filter(|l| !l.is_empty()).count()).unwrap_or(0);
    info
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Progress {
    Step { index: usize, total: usize, label: String },
    Line { text: String },
    Done { ok: bool, message: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PublishReport {
    pub version: String,
    pub files_published: usize,
    pub files_changed: usize,
    pub committed: bool,
    pub pushed: bool,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct PublishOptions {
    pub version: String,
    pub push: bool,
    pub excluded: HashSet<String>,
}

pub fn release_dir(cfg: &LauncherConfig) -> PathBuf {
    Path::new(&cfg.project_dir).join(&cfg.release_dir)
}

pub fn manifest_file(cfg: &LauncherConfig) -> PathBuf {
    Path::new(&cfg.project_dir).join(&cfg.manifest_path)
}

pub fn publish(cfg: &LauncherConfig, opts: &PublishOptions, cache: &mut ShaCache, on: &dyn Fn(Progress)) -> Result<PublishReport, String> {
    let total = if opts.push { 5 } else { 4 };
    let repo = Path::new(&cfg.project_dir);
    let step = |i: usize, label: &str| on(Progress::Step { index: i, total, label: label.to_string() });
    let line = |t: String| on(Progress::Line { text: t });
    let fail = |msg: String| {
        on(Progress::Done { ok: false, message: msg.clone() });
        msg
    };

    step(1, "Analyse des fichiers de release");
    let files = scan_release_dir(&release_dir(cfg), &cfg.ignore_suffixes, cache).map_err(fail)?;
    let previous = read_manifest(&manifest_file(cfg));
    let diffs = compare(previous.as_ref(), &files);
    let changed = diffs.iter().filter(|d| d.status != FileStatus::Unchanged && !opts.excluded.contains(&d.path)).count();
    line(format!("{} fichiers, {} modifiés depuis le dernier manifest", files.len(), changed));

    step(2, "Écriture du manifest");
    let manifest = build_manifest(&files, &opts.excluded, &cfg.base_url, &opts.version);
    let unchanged = previous.as_ref().map(|p| p.files == manifest.files && p.base_url == manifest.base_url).unwrap_or(false);
    if unchanged {
        line("Manifest inchangé, fichier conservé".into());
    } else {
        write_manifest(&manifest_file(cfg), &manifest).map_err(fail)?;
        line(format!("{} entrées → {}", manifest.files.len(), cfg.manifest_path));
    }

    step(3, "git add");
    let code = run_git(repo, &["add", "-A", "--", &cfg.release_dir, &cfg.manifest_path], &line).map_err(fail)?;
    if code != 0 {
        return Err(fail(format!("git add a échoué (code {code})")));
    }

    step(4, "git commit");
    let staged = run_git(repo, &["diff", "--cached", "--quiet"], &line).map_err(fail)?;
    let mut committed = false;
    if staged == 1 {
        let date = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();
        let msg = cfg.commit_message.replace("{date}", &date).replace("{version}", &opts.version);
        let code = run_git(repo, &["commit", "-m", &msg], &line).map_err(fail)?;
        if code != 0 {
            return Err(fail(format!("git commit a échoué (code {code})")));
        }
        committed = true;
    } else {
        line("Rien de nouveau à committer".into());
    }

    let mut pushed = false;
    if opts.push {
        step(5, "git push");
        let code = run_git(repo, &["push", "origin", &cfg.branch], &line).map_err(fail)?;
        if code != 0 {
            return Err(fail(format!("git push a échoué (code {code}) : vérifie les identifiants et la branche")));
        }
        pushed = true;
    }
    let message = match (committed, pushed) {
        (true, true) => "Modpack publié sur GitHub".to_string(),
        (true, false) => "Commit créé, rien n'a été poussé".to_string(),
        (false, true) => "Rien de nouveau, branche poussée".to_string(),
        (false, false) => "Rien à publier".to_string(),
    };
    on(Progress::Done { ok: true, message: message.clone() });
    Ok(PublishReport {
        version: opts.version.clone(),
        files_published: manifest.files.len(),
        files_changed: changed,
        committed,
        pushed,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("panel-pub-{}", crate::config::new_id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scans_hashes_and_compares() {
        let dir = tmp();
        fs::create_dir_all(dir.join("mods")).unwrap();
        fs::write(dir.join("mods/a.jar"), b"aaa").unwrap();
        fs::write(dir.join("mods/b.jar.disabled"), b"bbb").unwrap();
        fs::write(dir.join("note.txt"), b"n").unwrap();
        let mut cache = ShaCache::default();
        let files = scan_release_dir(&dir, &[".disabled".into()], &mut cache).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].rel_path, "mods/a.jar");
        assert_eq!(files[0].sha1, "7e240de74fb1ed08fa08d38063f6a6a91462a815");
        let m = build_manifest(&files, &HashSet::new(), "https://raw.githubusercontent.com/u/r/main/release files/", "v1");
        assert_eq!(m.files[0].url, "https://raw.githubusercontent.com/u/r/main/release%20files/mods/a.jar");
        fs::write(dir.join("mods/a.jar"), b"changed").unwrap();
        fs::write(dir.join("mods/c.jar"), b"c").unwrap();
        fs::remove_file(dir.join("note.txt")).unwrap();
        let files2 = scan_release_dir(&dir, &[".disabled".into()], &mut cache).unwrap();
        let diffs = compare(Some(&m), &files2);
        let status = |p: &str| diffs.iter().find(|d| d.path == p).map(|d| d.status);
        assert_eq!(status("mods/a.jar"), Some(FileStatus::Changed));
        assert_eq!(status("mods/c.jar"), Some(FileStatus::New));
        assert_eq!(status("note.txt"), Some(FileStatus::Removed));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn publishes_into_local_repo_without_push() {
        let dir = tmp();
        let ok = Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
        if !ok {
            return;
        }
        assert_eq!(run_git(&dir, &["init", "-q"], &|_| {}).unwrap(), 0);
        run_git(&dir, &["config", "user.email", "t@t"], &|_| {}).unwrap();
        run_git(&dir, &["config", "user.name", "t"], &|_| {}).unwrap();
        fs::create_dir_all(dir.join("release files/mods")).unwrap();
        fs::write(dir.join("release files/mods/a.jar"), b"aaa").unwrap();
        let cfg = LauncherConfig { project_dir: dir.to_string_lossy().to_string(), base_url: "https://x/y".into(), ..Default::default() };
        let opts = PublishOptions { version: "v1".into(), push: false, excluded: HashSet::new() };
        let mut cache = ShaCache::default();
        let events = std::sync::Mutex::new(Vec::new());
        let report = publish(&cfg, &opts, &mut cache, &|p| events.lock().unwrap().push(p)).unwrap();
        assert!(report.committed);
        assert!(!report.pushed);
        assert_eq!(report.files_published, 1);
        assert!(matches!(events.lock().unwrap().last(), Some(Progress::Done { ok: true, .. })));
        let info = git_info(&dir);
        assert!(info.is_repo);
        assert_eq!(info.dirty_count, 0);
        // Deuxième publication sans changement : pas de commit.
        let report2 = publish(&cfg, &opts, &mut cache, &|_| {}).unwrap();
        assert!(!report2.committed);
        let _ = fs::remove_dir_all(dir);
    }
}
