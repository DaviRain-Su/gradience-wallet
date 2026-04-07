use crate::error::{GradienceError, Result};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceRole::Owner => "owner",
            WorkspaceRole::Admin => "admin",
            WorkspaceRole::Member => "member",
            WorkspaceRole::Viewer => "viewer",
        }
    }
}

pub struct WorkspaceService;

impl WorkspaceService {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_workspace(
        &self,
        db: &Pool<Sqlite>,
        name: &str,
        owner_id: &str,
    ) -> Result<String> {
        if name.trim().is_empty() {
            return Err(GradienceError::InvalidCredential("workspace name cannot be empty".into()));
        }
        let id = uuid::Uuid::new_v4().to_string();
        gradience_db::queries::create_workspace(db, &id, name, owner_id, "free")
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))?;

        // Also add owner as workspace member
        gradience_db::queries::add_workspace_member(db, &id, owner_id, "owner")
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))?;

        Ok(id)
    }

    pub async fn add_member(
        &self,
        db: &Pool<Sqlite>,
        workspace_id: &str,
        user_id: &str,
        role: WorkspaceRole,
    ) -> Result<()> {
        gradience_db::queries::add_workspace_member(db, workspace_id, user_id, role.as_str())
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))?;
        Ok(())
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
