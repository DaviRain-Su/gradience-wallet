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
    let role: gradience_core::team::workspace::WorkspaceRole = role.parse()?;
    svc.add_member(&ctx.db, &workspace_id, &user_id, role).await?;
    println!("Invited {} to workspace {} as {:?}", user_email, workspace_id, role);
    Ok(())
}

pub async fn budget_set(
    ctx: &AppContext,
    workspace_id: String,
    amount: String,
    token: String,
    chain_id: String,
    period: String,
) -> Result<()> {
    let wei = gradience_core::eth_to_wei(&amount)?;
    let svc = gradience_core::team::shared_budget::SharedBudgetService::new();
    svc.allocate_workspace_budget(
        &ctx.db, &workspace_id, wei, &token, &chain_id, &period,
    ).await?;
    println!(
        "Set workspace {} budget to {} {} on {} (period: {})",
        workspace_id, amount, token, chain_id, period
    );
    Ok(())
}

pub async fn budget_status(
    ctx: &AppContext,
    workspace_id: String,
    token: String,
    chain_id: String,
    period: String,
) -> Result<()> {
    let svc = gradience_core::team::shared_budget::SharedBudgetService::new();
    let remaining = svc.get_remaining_budget(
        &ctx.db, &workspace_id, &token, &chain_id, &period,
    ).await?;
    // rough conversion back to ETH string for display
    let eth = remaining as f64 / 1e18;
    println!(
        "Workspace {} remaining budget: {:.6} {} on {} (period: {})",
        workspace_id, eth, token, chain_id, period
    );
    Ok(())
}
