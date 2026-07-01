use std::path::PathBuf;

use serde::Serialize;

use crate::config::OnboardingRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceWriteStatus {
    Writable,
    ReadOnly {
        path: PathBuf,
        operation: &'static str,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRouteStatus {
    Available,
    Missing,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingNextAction {
    OpenSoloChat,
    ShowProviderSetup,
    OpenCorePairing,
    OpenChildJoin,
    OpenStaffWorkerHandoff,
    OpenServerHeadlessSetup,
    BlockedReadOnlyWorkspace,
    ShowDoctor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingExitReport {
    pub role: OnboardingRole,
    pub provider_route_status: ProviderRouteStatus,
    pub workspace_write_status: WorkspaceWriteStatus,
    pub next_action: OnboardingNextAction,
}

pub fn decide_next_action(
    role: OnboardingRole,
    workspace_write_status: WorkspaceWriteStatus,
    provider_route_status: ProviderRouteStatus,
) -> OnboardingExitReport {
    let next_action = match (&workspace_write_status, role, provider_route_status) {
        (WorkspaceWriteStatus::ReadOnly { .. }, _, _) => {
            OnboardingNextAction::BlockedReadOnlyWorkspace
        }
        (WorkspaceWriteStatus::Writable, OnboardingRole::AgentClusterCore, _) => {
            OnboardingNextAction::OpenCorePairing
        }
        (WorkspaceWriteStatus::Writable, OnboardingRole::AgentClusterChildWorker, _) => {
            OnboardingNextAction::OpenChildJoin
        }
        (WorkspaceWriteStatus::Writable, OnboardingRole::StaffOsWorker, _) => {
            OnboardingNextAction::OpenStaffWorkerHandoff
        }
        (WorkspaceWriteStatus::Writable, OnboardingRole::ServerVpsNode, _) => {
            OnboardingNextAction::OpenServerHeadlessSetup
        }
        (
            WorkspaceWriteStatus::Writable,
            OnboardingRole::SoloLocalNode,
            ProviderRouteStatus::Available,
        ) => OnboardingNextAction::OpenSoloChat,
        (
            WorkspaceWriteStatus::Writable,
            OnboardingRole::SoloLocalNode,
            ProviderRouteStatus::Missing,
        ) => OnboardingNextAction::ShowProviderSetup,
    };

    OnboardingExitReport {
        role,
        provider_route_status,
        workspace_write_status,
        next_action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn writable() -> WorkspaceWriteStatus {
        WorkspaceWriteStatus::Writable
    }

    fn read_only() -> WorkspaceWriteStatus {
        WorkspaceWriteStatus::ReadOnly {
            path: PathBuf::from("/tmp/blocked"),
            operation: "write session state",
            message: "permission denied".to_string(),
        }
    }

    #[test]
    fn onboarding_agent_cluster_core_opens_pairing_not_chat() {
        let report = decide_next_action(
            OnboardingRole::AgentClusterCore,
            writable(),
            ProviderRouteStatus::Missing,
        );

        assert_eq!(report.next_action, OnboardingNextAction::OpenCorePairing);
        assert_ne!(report.next_action, OnboardingNextAction::OpenSoloChat);
    }

    #[test]
    fn onboarding_child_worker_opens_join_not_chat() {
        let report = decide_next_action(
            OnboardingRole::AgentClusterChildWorker,
            writable(),
            ProviderRouteStatus::Missing,
        );

        assert_eq!(report.next_action, OnboardingNextAction::OpenChildJoin);
        assert_ne!(report.next_action, OnboardingNextAction::OpenSoloChat);
        assert_ne!(report.next_action, OnboardingNextAction::ShowProviderSetup);
    }

    #[test]
    fn onboarding_no_model_does_not_open_chat() {
        let report = decide_next_action(
            OnboardingRole::SoloLocalNode,
            writable(),
            ProviderRouteStatus::Missing,
        );

        assert_eq!(report.next_action, OnboardingNextAction::ShowProviderSetup);
        assert_ne!(report.next_action, OnboardingNextAction::OpenSoloChat);
    }

    #[test]
    fn read_only_workspace_blocks_chat_and_pairing() {
        let report = decide_next_action(
            OnboardingRole::AgentClusterCore,
            read_only(),
            ProviderRouteStatus::Available,
        );

        assert_eq!(
            report.next_action,
            OnboardingNextAction::BlockedReadOnlyWorkspace
        );
        assert_ne!(report.next_action, OnboardingNextAction::OpenSoloChat);
        assert_ne!(report.next_action, OnboardingNextAction::OpenCorePairing);
    }

    #[test]
    fn no_chat_hijack_after_all_non_chat_roles() {
        for role in [
            OnboardingRole::AgentClusterCore,
            OnboardingRole::AgentClusterChildWorker,
            OnboardingRole::StaffOsWorker,
            OnboardingRole::ServerVpsNode,
        ] {
            let report = decide_next_action(role, writable(), ProviderRouteStatus::Available);
            assert_ne!(report.next_action, OnboardingNextAction::OpenSoloChat);
        }
    }
}
