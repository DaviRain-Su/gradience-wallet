use crate::error::{GradienceError, Result};

#[derive(Debug, Clone)]
pub struct WorkspaceDescriptor {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub plan: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl WorkspaceRole {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "owner" => Ok(WorkspaceRole::Owner),
            "admin" => Ok(WorkspaceRole::Admin),
            "member" => Ok(WorkspaceRole::Member),
            "viewer" => Ok(WorkspaceRole::Viewer),
            _ => Err(GradienceError::InvalidCredential(format!("invalid role: {}", s))),
        }
    }

    pub fn can_manage_policies(&self) -> bool {
        matches!(self, WorkspaceRole::Owner | WorkspaceRole::Admin)
    }

    pub fn can_invite_members(&self) -> bool {
        matches!(self, WorkspaceRole::Owner | WorkspaceRole::Admin)
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceMemberDescriptor {
    pub workspace_id: String,
    pub user_id: String,
    pub role: WorkspaceRole,
}

pub struct WorkspaceService;

impl WorkspaceService {
    pub fn new() -> Self {
        Self
    }

    pub fn create_workspace(
        &self,
        name: &str,
        owner_id: &str,
    ) -> Result<WorkspaceDescriptor> {
        if name.trim().is_empty() {
            return Err(GradienceError::InvalidCredential("workspace name cannot be empty".into()));
        }
        Ok(WorkspaceDescriptor {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            owner_id: owner_id.into(),
            plan: "free".into(),
        })
    }

    pub fn add_member(
        &self,
        workspace_id: &str,
        user_id: &str,
        role: WorkspaceRole,
    ) -> Result<WorkspaceMemberDescriptor> {
        Ok(WorkspaceMemberDescriptor {
            workspace_id: workspace_id.into(),
            user_id: user_id.into(),
            role,
        })
    }

    pub fn check_role_permission(
        &self,
        role: &WorkspaceRole,
        action: &str,
    ) -> bool {
        match action {
            "set_policy" | "delete_policy" => role.can_manage_policies(),
            "invite_member" | "remove_member" => role.can_invite_members(),
            "create_wallet" | "sign_tx" => !matches!(role, WorkspaceRole::Viewer),
            "view_wallet" | "view_balance" => true,
            _ => false,
        }
    }
}
