//! Mesure CPU / RAM / réseau du système et des processus serveurs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{Networks, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ProcSample {
    /// Pourcentage du total machine (toutes les cœurs = 100).
    pub cpu: f32,
    pub mem: u64,
    pub alive: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Sample {
    pub ts: i64,
    pub cpu_total: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub net_rx_bps: f64,
    pub net_tx_bps: f64,
    pub processes: HashMap<u32, ProcSample>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct SystemInfo {
    pub os: String,
    pub cpu_brand: String,
    pub cpu_count: usize,
    pub mem_total: u64,
}

pub struct Monitor {
    sys: System,
    nets: Networks,
    last_net: Instant,
    cpu_count: usize,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let cpu_count = sys.cpus().len().max(1);
        let nets = Networks::new_with_refreshed_list();
        Self { sys, nets, last_net: Instant::now(), cpu_count }
    }

    /// `pids` : processus à mesurer ; avec `include_children`, leurs enfants directs
    /// sont ajoutés (serveur lancé via un script cmd).
    pub fn sample(&mut self, pids: &[u32], include_children: bool) -> Sample {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        let kind = ProcessRefreshKind::nothing().with_cpu().with_memory();
        if include_children && !pids.is_empty() {
            self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
        } else if !pids.is_empty() {
            let wanted: Vec<Pid> = pids.iter().map(|p| Pid::from_u32(*p)).collect();
            self.sys.refresh_processes_specifics(ProcessesToUpdate::Some(&wanted), true, kind);
        }
        let elapsed = self.last_net.elapsed().as_secs_f64().max(0.001);
        self.nets.refresh(true);
        self.last_net = Instant::now();
        let (mut rx, mut tx) = (0u64, 0u64);
        for (_, data) in self.nets.iter() {
            rx += data.received();
            tx += data.transmitted();
        }
        let mut processes = HashMap::new();
        for pid in pids {
            let p = Pid::from_u32(*pid);
            let mut sample = ProcSample::default();
            if let Some(proc_) = self.sys.process(p) {
                sample.alive = true;
                sample.cpu = proc_.cpu_usage() / self.cpu_count as f32;
                sample.mem = proc_.memory();
                if include_children {
                    for child in self.sys.processes().values().filter(|c| c.parent() == Some(p)) {
                        sample.cpu += child.cpu_usage() / self.cpu_count as f32;
                        sample.mem += child.memory();
                    }
                }
            }
            processes.insert(*pid, sample);
        }
        Sample {
            ts: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0),
            cpu_total: self.sys.global_cpu_usage(),
            mem_used: self.sys.used_memory(),
            mem_total: self.sys.total_memory(),
            net_rx_bps: rx as f64 / elapsed,
            net_tx_bps: tx as f64 / elapsed,
            processes,
        }
    }

    pub fn info(&self) -> SystemInfo {
        SystemInfo {
            os: System::long_os_version().unwrap_or_default(),
            cpu_brand: self.sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
            cpu_count: self.cpu_count,
            mem_total: self.sys.total_memory(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_self() {
        let mut m = Monitor::new();
        let me = std::process::id();
        std::thread::sleep(std::time::Duration::from_millis(250));
        let s = m.sample(&[me], false);
        assert!(s.mem_total > 0);
        assert!(s.processes[&me].alive);
        assert!(s.processes[&me].mem > 0);
        let info = m.info();
        assert!(info.cpu_count >= 1);
    }
}
