//! Lancement et pilotage d'un processus serveur (java) : stdin pour la console,
//! stdout/stderr lus ligne par ligne, arrêt propre puis kill de secours.

use crate::config::{LaunchTarget, LocalServer};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, PartialEq)]
pub struct LaunchCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

impl LaunchCommand {
    pub fn display(&self) -> String {
        let mut s = self.program.clone();
        for a in &self.args {
            s.push(' ');
            if a.contains(' ') {
                s.push('"');
                s.push_str(a);
                s.push('"');
            } else {
                s.push_str(a);
            }
        }
        s
    }
}

/// Découpe une ligne d'arguments en respectant les guillemets doubles.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    for ch in s.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    out.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            c => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        out.push(cur);
    }
    out
}

pub fn build_launch_command(local: &LocalServer) -> Result<LaunchCommand, String> {
    if local.dir.trim().is_empty() {
        return Err("Dossier du serveur non renseigné".into());
    }
    let cwd = PathBuf::from(&local.dir);
    if !cwd.is_dir() {
        return Err(format!("Dossier introuvable : {}", local.dir));
    }
    let java = if local.java_path.trim().is_empty() { "java".to_string() } else { local.java_path.trim().to_string() };
    let mut args = split_args(&local.jvm_args);
    let program = match &local.launch {
        LaunchTarget::Jar { path } => {
            if path.trim().is_empty() {
                return Err("Jar du serveur non renseigné".into());
            }
            args.push("-jar".into());
            args.push(path.trim().to_string());
            java
        }
        LaunchTarget::ArgsFile { path } => {
            if path.trim().is_empty() {
                return Err("Fichier d'arguments non renseigné".into());
            }
            args.push(format!("@{}", path.trim()));
            java
        }
        LaunchTarget::Script { path } => {
            if path.trim().is_empty() {
                return Err("Script de lancement non renseigné".into());
            }
            args.clear();
            if cfg!(windows) {
                args.push("/c".into());
                args.push(path.trim().to_string());
                "cmd".to_string()
            } else {
                args.push(path.trim().to_string());
                "sh".to_string()
            }
        }
    };
    args.extend(split_args(&local.extra_args));
    Ok(LaunchCommand { program, args, cwd })
}

pub type LineHandler = Arc<dyn Fn(String) + Send + Sync>;
pub type ExitHandler = Box<dyn FnOnce(Option<i32>) + Send>;

pub struct ManagedProcess {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    pub pid: u32,
    pub started: Instant,
    exited: AtomicBool,
    exit_code: Mutex<Option<i32>>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopOutcome {
    AlreadyExited,
    Clean,
    Killed,
}

pub fn spawn(cmd: &LaunchCommand, on_line: LineHandler, on_exit: ExitHandler) -> Result<Arc<ManagedProcess>, String> {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .current_dir(&cmd.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|e| format!("Impossible de lancer « {} » : {e}", cmd.program))?;
    let pid = child.id();
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let proc_ = Arc::new(ManagedProcess {
        child: Mutex::new(child),
        stdin: Mutex::new(stdin),
        pid,
        started: Instant::now(),
        exited: AtomicBool::new(false),
        exit_code: Mutex::new(None),
    });

    let readers_total = stdout.is_some() as usize + stderr.is_some() as usize;
    let readers_done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    if let Some(out) = stdout {
        let handler = on_line.clone();
        let done = readers_done.clone();
        thread::spawn(move || {
            read_lines(out, handler);
            done.fetch_add(1, Ordering::SeqCst);
        });
    }
    if let Some(err) = stderr {
        let handler = on_line.clone();
        let done = readers_done.clone();
        thread::spawn(move || {
            read_lines(err, handler);
            done.fetch_add(1, Ordering::SeqCst);
        });
    }

    let watched = proc_.clone();
    thread::spawn(move || {
        // La fin est détectée par try_wait : un petit-fils (java lancé par cmd)
        // peut garder les pipes ouverts bien après la mort du processus direct.
        let code = loop {
            let status = watched.child.lock().map(|mut c| c.try_wait().ok().flatten()).unwrap_or(None);
            match status {
                Some(s) => break s.code(),
                None => thread::sleep(Duration::from_millis(100)),
            }
        };
        // Laisse aux lecteurs le temps de vider les dernières lignes.
        let deadline = Instant::now() + Duration::from_secs(2);
        while readers_done.load(Ordering::SeqCst) < readers_total && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        if let Ok(mut ec) = watched.exit_code.lock() {
            *ec = code;
        }
        watched.exited.store(true, Ordering::SeqCst);
        if let Ok(mut s) = watched.stdin.lock() {
            *s = None;
        }
        on_exit(code);
    });

    Ok(proc_)
}

fn read_lines<R: std::io::Read>(reader: R, handler: LineHandler) {
    let mut buf = BufReader::new(reader);
    let mut bytes = Vec::with_capacity(512);
    loop {
        bytes.clear();
        match buf.read_until(b'\n', &mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&bytes);
                handler(line.trim_end_matches(['\r', '\n']).to_string());
            }
        }
    }
}

impl ManagedProcess {
    pub fn is_running(&self) -> bool {
        !self.exited.load(Ordering::SeqCst)
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code.lock().ok().and_then(|c| *c)
    }

    pub fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn write_line(&self, line: &str) -> Result<(), String> {
        let mut guard = self.stdin.lock().map_err(|_| "stdin verrouillé".to_string())?;
        let stdin = guard.as_mut().ok_or_else(|| "Le processus n'accepte plus d'entrée".to_string())?;
        stdin
            .write_all(format!("{}\n", line.trim_end()).as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|e| format!("Écriture console : {e}"))
    }

    /// Tue le processus et, sous Windows, tout son arbre (cmd → java).
    pub fn kill(&self) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = Command::new("taskkill")
                .args(["/T", "/F", "/PID", &self.pid.to_string()])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        if let Ok(mut c) = self.child.lock() {
            let _ = c.kill();
        }
    }

    /// Attend la fin du processus, vrai s'il s'est terminé dans le délai.
    pub fn wait_exit(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while self.is_running() {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(100));
        }
        true
    }

    /// Envoie "stop", attend, kill si le serveur ne répond plus.
    pub fn graceful_stop(&self, timeout: Duration) -> StopOutcome {
        if !self.is_running() {
            return StopOutcome::AlreadyExited;
        }
        if self.write_line("stop").is_err() {
            self.kill();
            self.wait_exit(Duration::from_secs(5));
            return StopOutcome::Killed;
        }
        if self.wait_exit(timeout) {
            StopOutcome::Clean
        } else {
            self.kill();
            self.wait_exit(Duration::from_secs(10));
            StopOutcome::Killed
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DetectedLaunch {
    pub launch: Option<LaunchTarget>,
    pub loader: String,
    pub mc_version: String,
    pub jvm_args: String,
    pub notes: Vec<String>,
}

/// Devine comment lancer un serveur à partir de son dossier.
pub fn detect_launch(dir: &Path) -> DetectedLaunch {
    let mut d = DetectedLaunch::default();
    if !dir.is_dir() {
        d.notes.push("Dossier introuvable".into());
        return d;
    }
    if let Some((rel, loader, mc)) = find_args_file(dir) {
        d.launch = Some(LaunchTarget::ArgsFile { path: rel });
        d.loader = loader;
        d.mc_version = mc;
    }
    let mut jars: Vec<String> = fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .filter(|n| n.to_ascii_lowercase().ends_with(".jar"))
                .collect()
        })
        .unwrap_or_default();
    jars.sort();
    if d.launch.is_none() {
        if let Some(j) = jars.iter().find(|j| j.starts_with("fabric-server")) {
            d.launch = Some(LaunchTarget::Jar { path: j.clone() });
            d.loader = "fabric".into();
            d.mc_version = extract_mc_version(j);
        } else if let Some(j) = jars
            .iter()
            .find(|j| j == &"server.jar" || j.starts_with("minecraft_server") || j.starts_with("paper") || j.starts_with("purpur"))
        {
            d.launch = Some(LaunchTarget::Jar { path: j.clone() });
            d.loader = if j.starts_with("paper") || j.starts_with("purpur") { "paper".into() } else { "vanilla".into() };
            d.mc_version = extract_mc_version(j);
        } else if let Some(j) = jars.iter().find(|j| j.starts_with("forge-")) {
            d.launch = Some(LaunchTarget::Jar { path: j.clone() });
            d.loader = "forge".into();
            d.mc_version = extract_mc_version(j);
        } else if let Some(j) = jars.first() {
            d.launch = Some(LaunchTarget::Jar { path: j.clone() });
            d.notes.push("Jar choisi par défaut, à vérifier".into());
        }
    }
    if d.launch.is_none() {
        for script in ["run.bat", "start.bat", "start.sh", "run.sh"] {
            if dir.join(script).is_file() {
                d.launch = Some(LaunchTarget::Script { path: script.into() });
                d.notes.push("Lancement via script : la console marche, l'arrêt forcé ne tue que cmd".into());
                break;
            }
        }
    }
    if let Ok(text) = fs::read_to_string(dir.join("user_jvm_args.txt")) {
        let args: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
        if !args.is_empty() {
            d.jvm_args = args.join(" ");
        }
    }
    if d.jvm_args.is_empty() {
        for script in ["run.bat", "start.bat"] {
            if let Ok(text) = fs::read_to_string(dir.join(script)) {
                if let Some(a) = jvm_args_from_script(&text) {
                    d.jvm_args = a;
                    break;
                }
            }
        }
    }
    if d.launch.is_none() {
        d.notes.push("Aucun jar ni script reconnu".into());
    }
    d
}

fn find_args_file(dir: &Path) -> Option<(String, String, String)> {
    let candidates = [
        ("libraries/net/neoforged/neoforge", "neoforge"),
        ("libraries/net/minecraftforge/forge", "forge"),
    ];
    for (rel, loader) in candidates {
        let base = dir.join(rel);
        let Ok(rd) = fs::read_dir(&base) else { continue };
        let mut versions: Vec<String> =
            rd.flatten().filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().to_str().map(String::from)).collect();
        versions.sort();
        for v in versions.iter().rev() {
            let name = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
            if base.join(v).join(name).is_file() {
                let mc = if loader == "neoforge" { neoforge_to_mc(v) } else { v.split('-').next().unwrap_or("").to_string() };
                return Some((format!("{rel}/{v}/{name}"), loader.to_string(), mc));
            }
        }
    }
    None
}

/// NeoForge 21.1.72 → Minecraft 1.21.1 ; 20.4.x → 1.20.4.
fn neoforge_to_mc(v: &str) -> String {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() >= 2 {
        if parts[1] == "0" {
            format!("1.{}", parts[0])
        } else {
            format!("1.{}.{}", parts[0], parts[1])
        }
    } else {
        String::new()
    }
}

fn extract_mc_version(name: &str) -> String {
    let re = regex::Regex::new(r"(?:^|[^\d])(1\.\d{1,2}(?:\.\d{1,2})?|2[6-9]\.\d{1,2}(?:\.\d{1,2})?)").unwrap();
    re.captures(name).map(|c| c[1].to_string()).unwrap_or_default()
}

fn jvm_args_from_script(text: &str) -> Option<String> {
    for line in text.lines() {
        let l = line.trim();
        if l.to_ascii_lowercase().contains("java") && l.contains("-Xm") {
            let args: Vec<String> =
                split_args(l).into_iter().filter(|a| a.starts_with("-X") || a.starts_with("-D") || a.starts_with("-XX")).collect();
            if !args.is_empty() {
                return Some(args.join(" "));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn splits_quoted_args() {
        assert_eq!(split_args(r#"-Xmx4G "-Dfoo=a b" nogui"#), vec!["-Xmx4G", "-Dfoo=a b", "nogui"]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn builds_commands() {
        let dir = std::env::temp_dir();
        let mut local = LocalServer { dir: dir.to_string_lossy().to_string(), jvm_args: "-Xmx1G".into(), ..Default::default() };
        let cmd = build_launch_command(&local).unwrap();
        assert_eq!(cmd.program, "java");
        assert_eq!(cmd.args, vec!["-Xmx1G", "-jar", "server.jar", "nogui"]);
        local.launch = LaunchTarget::ArgsFile { path: "libraries/x/win_args.txt".into() };
        let cmd = build_launch_command(&local).unwrap();
        assert_eq!(cmd.args, vec!["-Xmx1G", "@libraries/x/win_args.txt", "nogui"]);
        local.dir = "/definitely/missing".into();
        assert!(build_launch_command(&local).is_err());
    }

    #[test]
    fn detects_neoforge_layout() {
        let dir = std::env::temp_dir().join(format!("panel-detect-{}", crate::config::new_id()));
        let args = dir.join("libraries/net/neoforged/neoforge/21.1.72");
        fs::create_dir_all(&args).unwrap();
        fs::write(args.join("win_args.txt"), "-p x").unwrap();
        fs::write(args.join("unix_args.txt"), "-p x").unwrap();
        fs::write(dir.join("user_jvm_args.txt"), "# comment\n-Xmx8G\n-Xms2G\n").unwrap();
        let d = detect_launch(&dir);
        assert_eq!(d.loader, "neoforge");
        assert_eq!(d.mc_version, "1.21.1");
        assert_eq!(d.jvm_args, "-Xmx8G -Xms2G");
        assert!(matches!(d.launch, Some(LaunchTarget::ArgsFile { .. })));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn detects_fabric_jar() {
        let dir = std::env::temp_dir().join(format!("panel-detect-{}", crate::config::new_id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("fabric-server-mc.1.21.1-loader.0.16.5-launcher.1.0.1.jar"), "").unwrap();
        let d = detect_launch(&dir);
        assert_eq!(d.loader, "fabric");
        assert_eq!(d.mc_version, "1.21.1");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn spawn_reads_lines_and_exits() {
        let dir = std::env::temp_dir();
        let cmd = if cfg!(windows) {
            LaunchCommand { program: "cmd".into(), args: vec!["/c".into(), "echo hello & echo world".into()], cwd: dir }
        } else {
            LaunchCommand { program: "sh".into(), args: vec!["-c".into(), "echo hello; echo world".into()], cwd: dir }
        };
        let count = Arc::new(AtomicUsize::new(0));
        let c2 = count.clone();
        let exited = Arc::new(AtomicBool::new(false));
        let e2 = exited.clone();
        let p = spawn(
            &cmd,
            Arc::new(move |_l| {
                c2.fetch_add(1, Ordering::SeqCst);
            }),
            Box::new(move |_code| e2.store(true, Ordering::SeqCst)),
        )
        .unwrap();
        assert!(p.wait_exit(Duration::from_secs(10)));
        // Laisse le thread de surveillance appeler on_exit.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !exited.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(exited.load(Ordering::SeqCst));
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert_eq!(p.graceful_stop(Duration::from_secs(1)), StopOutcome::AlreadyExited);
    }

    #[cfg(not(windows))]
    #[test]
    fn graceful_stop_kills_unresponsive() {
        let cmd = LaunchCommand { program: "sh".into(), args: vec!["-c".into(), "sleep 30".into()], cwd: std::env::temp_dir() };
        let p = spawn(&cmd, Arc::new(|_| {}), Box::new(|_| {})).unwrap();
        assert!(p.is_running());
        assert_eq!(p.graceful_stop(Duration::from_millis(300)), StopOutcome::Killed);
        assert!(!p.is_running());
    }
}
