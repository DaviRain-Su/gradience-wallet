use crate::error::{GradienceError, Result};
use sha3::{Sha3_256, Digest};

#[derive(Debug, Clone)]
pub struct ApiKeyDescriptor {
    pub id: String,
    pub wallet_id: String,
    pub name: String,
    pub raw_token: Option<String>,
    pub token_hash: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[allow(dead_code)]
fn generate_token() -> String {
    format!("ows_key_{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
}

pub struct ApiKeyService;

impl ApiKeyService {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn create_key(&self, wallet_id: &str, name: &str) -> Result<ApiKeyDescriptor> {
        if name.trim().is_empty() {
            return Err(GradienceError::InvalidCredential("api key name cannot be empty".into()));
        }
        let token = format!("ows_key_{:064x}", uuid::Uuid::new_v4().simple());
        let hash = format!("{:x}", Sha3_256::digest(token.as_bytes()));
        Ok(ApiKeyDescriptor {
            id: uuid::Uuid::new_v4().to_string(),
            wallet_id: wallet_id.into(),
            name: name.into(),
            raw_token: Some(token),
            token_hash: hash,
            permissions: vec!["sign".into(), "read".into()],
            expires_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    
    pub async fn verify_key(&self, raw_token: &str, descriptor: &ApiKeyDescriptor) -> Result<bool> {
        if descriptor.raw_token.is_none() {
            return Err(GradienceError::InvalidCredential("no raw token stored".into()));
        }
        let computed = format!("{:x}", Sha3_256::digest(raw_token.as_bytes()));
        Ok(computed == descriptor.token_hash)
    }
}


