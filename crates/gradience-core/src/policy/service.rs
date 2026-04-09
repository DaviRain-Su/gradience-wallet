use crate::error::{GradienceError, Result};
use std::path::Path;

#[allow(dead_code)]
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

    let ows_policy_result: std::result::Result<ows_core::Policy, _> =
        serde_json::from_value(policy_value.clone());

    let (policy_id_out, name_out) = match ows_policy_result {
        Ok(ows_policy) => {
            // Write to OWS vault if compatible
            if let Some(vp) = vault_path {
                let _ = ows_lib::policy_store::save_policy(&ows_policy, Some(vp));
            }
            (ows_policy.id, ows_policy.name)
        }
        Err(_) => {
            // Gradience-only policy: skip OWS, generate id/name from JSON
            let id = policy_value
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&policy_id)
                .to_string();
            let name = policy_value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("gradience-policy")
                .to_string();
            (id, name)
        }
    };

    // Write to Gradience DB
    gradience_db::queries::create_policy(
        pool,
        &policy_id_out,
        &name_out,
        wallet_id,
        workspace_id,
        content,
        1,
    )
    .await
    .map_err(|e| GradienceError::Database(e.to_string()))?;

    Ok(policy_id_out)
}
