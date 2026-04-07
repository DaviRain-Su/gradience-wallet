use gradience_core::ows::local_adapter::LocalOwsAdapter;
use sqlx::{Pool, Sqlite};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppContext {
    pub db: Pool<Sqlite>,
    pub ows: Arc<LocalOwsAdapter>,
    pub vault_dir: PathBuf,
}

impl AppContext {
    pub async fn new(db_path: &str, vault_dir: PathBuf) -> anyhow::Result<Self> {
        // Ensure parent directory exists so SQLite can create the file
        let db_file = db_path.trim_start_matches("sqlite:").split('?').next().unwrap_or(db_path);
        if let Some(parent) = PathBuf::from(db_file).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&vault_dir)?;

        let db = sqlx::SqlitePool::connect(db_path).await?;
        let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/gradience-db/migrations");
        let migrator = sqlx::migrate::Migrator::new(migrations_path).await?;
        migrator.run(&db).await?;
        let ows = Arc::new(LocalOwsAdapter::new(vault_dir.clone()));
        Ok(Self { db, ows, vault_dir })
    }
}
