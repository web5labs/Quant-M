use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostPlatform {
    AndroidTermux,
    Macos,
    Linux,
    Windows,
    Unknown,
}

impl HostPlatform {
    pub fn detect() -> Self {
        if cfg!(target_os = "android") || std::env::var_os("TERMUX_VERSION").is_some() {
            Self::AndroidTermux
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Unknown
        }
    }
}

impl fmt::Display for HostPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AndroidTermux => write!(f, "android_termux"),
            Self::Macos => write!(f, "macos"),
            Self::Linux => write!(f, "linux"),
            Self::Windows => write!(f, "windows"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalSurface {
    TermuxWindows,
    Cmux,
    Tmux,
    PlainTerminal,
}

impl fmt::Display for TerminalSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TermuxWindows => write!(f, "termux_windows"),
            Self::Cmux => write!(f, "cmux"),
            Self::Tmux => write!(f, "tmux"),
            Self::PlainTerminal => write!(f, "plain_terminal"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CockpitLaneInput {
    pub repo_path: PathBuf,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CockpitLanePlan {
    pub lane_id: String,
    pub repo_path: PathBuf,
    pub model: Option<String>,
    pub quant_m_command: String,
    pub launcher_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalCockpitPlan {
    pub host_platform: HostPlatform,
    pub terminal_surface: TerminalSurface,
    pub workspace_dir: PathBuf,
    pub shared_state_sqlite: PathBuf,
    pub shared_state_hot_store: PathBuf,
    pub session_dir: PathBuf,
    pub lanes: Vec<CockpitLanePlan>,
    pub launch_policy: String,
    pub shared_state_policy: String,
}

pub fn plan_terminal_cockpit(
    cfg: &Config,
    host_platform: HostPlatform,
    lane_inputs: Vec<CockpitLaneInput>,
) -> TerminalCockpitPlan {
    let terminal_surface = surface_for_host(host_platform);
    let lanes = normalize_lanes(cfg, terminal_surface, lane_inputs);

    TerminalCockpitPlan {
        host_platform,
        terminal_surface,
        workspace_dir: cfg.workspace_dir.clone(),
        shared_state_sqlite: cfg.state_sql.sqlite_path.clone(),
        shared_state_hot_store: cfg.workspace_dir.join("state").join("shared-state.redb"),
        session_dir: cfg.runtime.session_dir.clone(),
        lanes,
        launch_policy:
            "planning_only: Quant-M emits launcher previews; it does not spawn terminals"
                .to_string(),
        shared_state_policy:
            "Quant-M owns shared state and session evidence; terminal surfaces only host lanes"
                .to_string(),
    }
}

pub fn surface_for_host(host_platform: HostPlatform) -> TerminalSurface {
    match host_platform {
        HostPlatform::AndroidTermux => TerminalSurface::TermuxWindows,
        HostPlatform::Macos => TerminalSurface::Cmux,
        HostPlatform::Linux | HostPlatform::Windows => TerminalSurface::Tmux,
        HostPlatform::Unknown => TerminalSurface::PlainTerminal,
    }
}

fn normalize_lanes(
    cfg: &Config,
    terminal_surface: TerminalSurface,
    lane_inputs: Vec<CockpitLaneInput>,
) -> Vec<CockpitLanePlan> {
    let inputs = if lane_inputs.is_empty() {
        vec![CockpitLaneInput {
            repo_path: cfg.workspace_dir.clone(),
            model: cfg
                .preferences
                .preferred_openrouter_model
                .clone()
                .or_else(|| {
                    cfg.preferences
                        .preferred_local_model
                        .as_ref()
                        .map(|m| m.model.clone())
                }),
        }]
    } else {
        lane_inputs
    };

    inputs
        .into_iter()
        .enumerate()
        .map(|(index, input)| {
            let lane_number = index + 1;
            let lane_id = format!("lane-{lane_number:02}");
            let quant_m_command =
                "quant-m status && quant-m session list && quant-m state list".to_string();
            let launcher_preview = launcher_preview(
                terminal_surface,
                &lane_id,
                &input.repo_path,
                &quant_m_command,
            );
            CockpitLanePlan {
                lane_id,
                repo_path: input.repo_path,
                model: input.model,
                quant_m_command,
                launcher_preview,
            }
        })
        .collect()
}

fn launcher_preview(
    terminal_surface: TerminalSurface,
    lane_id: &str,
    repo_path: &std::path::Path,
    quant_m_command: &str,
) -> String {
    let repo = repo_path.display();
    match terminal_surface {
        TerminalSurface::TermuxWindows => {
            format!("termux-new-session -s {lane_id} 'cd {repo} && {quant_m_command}'")
        }
        TerminalSurface::Cmux => {
            format!(
                "cmux new-surface --type terminal --title {lane_id} --command 'cd {repo} && {quant_m_command}'"
            )
        }
        TerminalSurface::Tmux => {
            format!("tmux new-window -n {lane_id} 'cd {repo} && {quant_m_command}'")
        }
        TerminalSurface::PlainTerminal => {
            format!("cd {repo} && {quant_m_command}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HostPlatform, TerminalSurface, surface_for_host};
    use crate::config::Config;

    #[test]
    fn host_platform_maps_to_expected_terminal_surface() {
        assert_eq!(
            surface_for_host(HostPlatform::AndroidTermux),
            TerminalSurface::TermuxWindows
        );
        assert_eq!(surface_for_host(HostPlatform::Macos), TerminalSurface::Cmux);
        assert_eq!(surface_for_host(HostPlatform::Linux), TerminalSurface::Tmux);
        assert_eq!(
            surface_for_host(HostPlatform::Windows),
            TerminalSurface::Tmux
        );
        assert_eq!(
            surface_for_host(HostPlatform::Unknown),
            TerminalSurface::PlainTerminal
        );
    }

    #[test]
    fn default_plan_keeps_shared_state_in_quant_m() {
        let cfg = Config::default();
        let plan = super::plan_terminal_cockpit(&cfg, HostPlatform::Macos, vec![]);

        assert_eq!(plan.terminal_surface, TerminalSurface::Cmux);
        assert_eq!(plan.lanes.len(), 1);
        assert_eq!(plan.shared_state_sqlite, cfg.state_sql.sqlite_path);
        assert!(plan.lanes[0].launcher_preview.starts_with("cmux "));
        assert!(
            plan.shared_state_policy
                .contains("Quant-M owns shared state")
        );
    }
}
