use crate::context::AppContext;
use anyhow::Result;

pub async fn create_workspace(ctx: &AppContext, name: String) -> Result<()> {
    let svc = gradience_core::team::workspace::WorkspaceService::new();
    let id = svc.create_workspace(&ctx.db, &name, "user-1").await?;
    println!("Created workspace '{}' (id: {})", name, id);
    Ok(())
}

pub async fn invite(ctx: &AppContext, workspace_id: String, user_email: String, role: String) -> Result<()> {
    // Ensure user exists by email (create if not)
    let user = gradience_db::queries::get_user_by_email(&ctx.db, &user_email).await?;
    let user_id = match user {
        Some(u) => u.id,
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            gradience_db::queries::create_user(&ctx.db, &id, &user_email).await?;
            id
        }
    };

    let svc = gradience_core::team::workspace::WorkspaceService::new();
    let role = gradience_core::team::workspace::WorkspaceRole::from_str(&role)?;
    svc.add_member(&ctx.db, &workspace_id, &user_id, role).await?;
    println!("Invited {} to workspace {} as {:?}", user_email, workspace_id, role);
    Ok(())
}
