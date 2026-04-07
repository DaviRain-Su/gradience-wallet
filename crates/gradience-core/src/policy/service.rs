use crate::error::{GradienceError, Result};
use std::path::Path;

fn map_ows_err(e: ows_lib::OwsLibError) -> GradienceError {
    GradienceError::Ows(e.to_string())
}

/// Parse policy JSON and synchronise to both Gradience DB and OWS vault.
///
/// Accepts either a full OWS Policy object, or a minimal `{ "rules": [...] }`
/// object from which we auto-generate the remaining fields.
pub async fn create_policy_sync(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    wallet_id: Option<&str>,
    workspace_id: Option<&str>,
    content: &str,
    vault_path: Option<&Path>,
) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| GradienceError::InvalidCredential(format!("Invalid policy JSON: {}", e)))?;

    let policy_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Try to interpret the uploaded JSON as an OWS Policy.
    // If it already has "id" / "name" keep them; otherwise generate them.
    let mut policy_value = value.clone();
    if policy_value.get("id").is_none() {
        policy_value["id"] = serde_json::json!(&policy_id);
    }
    if policy_value.get("name").is_none() {
        policy_value["name"] = "gradience-policy".into();
    }
    if policy_value.get("version").is_none() {
        policy_value["version"] = 1.into();
    }
    if policy_value.get("created_at").is_none() {
        policy_value["created_at"] = serde_json::json!(&now);
    }
    if policy_value.get("action").is_none() {
        policy_value["action"] = "deny".into();
    }

    let ows_policy: ows_core::Policy = serde_json::from_value(policy_value.clone())
        .map_err(|e| GradienceError::InvalidCredential(format!("Policy not compatible with OWS: {}", e)))?;

    // Write to OWS vault
    ows_lib::policy_store::save_policy(&ows_policy, vault_path)
        .map_err(map_ows_err)?;

    // Write to Gradience DB
    let name = ows_policy.name.clone();
    gradience_db::queries::create_policy(
        pool,
        &ows_policy.id,
        &name,
        wallet_id,
        workspace_id,
        content,
        1,
    )
    .await
    .map_err(|e| GradienceError::Database(e.to_string()))?;

    Ok(ows_policy.id)
}
