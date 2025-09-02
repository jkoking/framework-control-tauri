use crate::config; // for save/load
use crate::state::AppState;
use crate::types::{Config, PartialConfig, SystemInfoEnvelope};
use sysinfo::System;
use tracing::{error, info};
use anyhow::Result;

pub struct Api {
    state: AppState,
}

impl Api {
pub(super) async fn new() -> Self {
        Self { state: crate::state::AppState::initialize().await  }
    }

    /// Health: returns overall service health and CLI presence
    pub(super) async fn health(&self) -> bool {
        self.state.cli.is_some()
    }

    /// Power info
    pub(super) async fn get_power(&self) -> Result<String, String> {
        match &self.state.cli {
            Some(cli) => match cli.power().await {
                Ok(output) => Ok(output),
                Err(e) => Err(e),
            },
            None => return Err("framework_tool not found".into()),
        }
    }

    /// Thermal info
    pub(super) async fn get_thermal(&self) -> Result<String, String> {
        match &self.state.cli {
            Some(cli) => match cli.thermal().await {
                Ok(output) => Ok(output),
                Err(e) => Err(e),
            },
            None => return Err("framework_tool not found".into()),
        }
    }

    /// Versions
    pub(super) async fn get_versions(&self) -> Result<String, String> {
        match &self.state.cli {
            Some(cli) => match cli.versions().await {
                Ok(output) => Ok(output),
                Err(e) => Err(e),
            },
            None => return Err("framework_tool not found".into()),
        }
    }

    /// Get config
    pub(super) async fn get_config(&self) -> Config {
        self.state.config.read().await.clone()
    }

    /// Set config (partial)
    async fn set_config(&self,req: PartialConfig,) -> Result<(), String> {
        let mut merged = self.state.config.read().await.clone();
        if let Some(fan) = req.fan {
            let mut new_fan = merged.fan.clone();
            // Overwrite sections only if provided
            if let Some(m) = fan.mode { new_fan.mode = m; }
            if let Some(man) = fan.manual { new_fan.manual = Some(man); }
            if let Some(cur) = fan.curve { new_fan.curve = Some(cur); }
            if let Some(cal) = fan.calibration { new_fan.calibration = Some(cal); }
            merged.fan = new_fan;
        }
        if let Err(e) = config::save(&merged) {
            error!("config save error: {}", e);
            return Err(format!("config save error: {}", e));
        }
        {
            let mut w = self.state.config.write().await;
            *w = merged;
        }
        info!("set_config applied successfully");
        return Ok(())
    }

    /// System info
    async fn get_system_info(&self) -> SystemInfoEnvelope {
        let sys = System::new_all();
        let mut cpu = sys.global_cpu_info().brand().trim().to_string();
        if cpu.is_empty() {
            if let Some(c) = sys.cpus().iter().find(|c| !c.brand().trim().is_empty()) {
                cpu = c.brand().trim().to_string();
            }
        }
        let mem_mb = sys.total_memory() / 1024 / 1024;
        let os = System::name().unwrap_or_else(|| "Unknown OS".into());
        let dgpu = pick_dedicated_gpu(&get_gpu_names().await);
        SystemInfoEnvelope {
            ok: true,
            cpu,
            memory_total_mb: mem_mb,
            os,
            dgpu,
        }
    }
}

async fn get_gpu_names() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        use tokio::process::Command;
        let ps = "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name";
        if let Ok(out) = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(ps)
            .output()
            .await
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                return s
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

fn pick_dedicated_gpu(names: &[String]) -> Option<String> {
    let mut best: Option<String> = None;
    for n in names {
        let lo = n.to_ascii_lowercase();
        let looks_discrete = lo.contains("rtx")
            || lo.contains("gtx")
            || lo.contains("rx ")
            || lo.contains("arc ")
            || lo.contains("radeon pro")
            || lo.contains("geforce")
            || lo.contains("quadro")
            || lo.contains("radeon rx");
        let looks_integrated =
            lo.contains("uhd") || lo.contains("iris") || lo.contains("vega") || lo.contains("780m");
        if looks_discrete && !looks_integrated {
            return Some(n.clone());
        }
        if best.is_none() && !looks_integrated {
            best = Some(n.clone());
        }
    }
    best
}
