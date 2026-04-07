use crate::context::AppContext;
use anyhow::Result;

pub async fn create_workspace(_ctx: &AppContext, name: String) -> Result<()> {
    let svc = gradience_core::team::workspace::WorkspaceService::new();
    let ws = svc.create_workspace(&name, "user-1")?;
    println!("Created workspace {} (id: {})", ws.name, ws.id);
    Ok(())
}

pub async fn invite(_ctx: &AppContext, workspace_id: String, user_email: String, role: String) -> Result<()> {
    let svc = gradience_core::team::workspace::WorkspaceService::new();
    let role = gradience_core::team::workspace::WorkspaceRole::from_str(&role)?;
    let member = svc.add_member(&workspace_id, &user_email, role)?;
    println!("Invited {} to workspace {} as {:?}", member.user_id, member.workspace_id, member.role);
    Ok(())
}
