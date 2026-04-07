use gradience_core::ows::local_adapter::LocalOwsAdapter;
use sqlx::{Pool, Sqlite};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppContext {
    pub db: Pool<Sqlite>,
    pub ows: Arc<LocalOwsAdapter>,
    pub vault_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl AppContext {
    pub async fn new(db_path: &str, data_dir: PathBuf, vault_dir: PathBuf) -> anyhow::Result<Self> {
        let db_file = db_path.trim_start_matches("sqlite:").split('?').next().unwrap_or(db_path);
        if let Some(parent) = PathBuf::from(db_file).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&vault_dir)?;

        let db = sqlx::SqlitePool::connect(db_path).await?;
        let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/gradience-db/migrations");
        let migrator = sqlx::migrate::Migrator::new(migrations_path).await?;
        migrator.run(&db).await?;
        let ows = Arc::new(LocalOwsAdapter::new(vault_dir.clone()));
        Ok(Self { db, ows, vault_dir, data_dir })
    }

    pub fn session_path(&self) -> PathBuf {
        self.data_dir.join(".session")
    }

    pub fn read_passphrase(&self) -> Option<String> {
        let path = self.session_path();
        if path.exists() {
            fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
        } else {
            None
        }
    }

    pub fn write_passphrase(&self, passphrase: &str) -> anyhow::Result<()> {
        let path = self.session_path();
        fs::write(&path, passphrase)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(&path, perms);
        }
        Ok(())
    }
}
