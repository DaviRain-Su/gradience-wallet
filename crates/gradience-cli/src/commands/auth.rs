use crate::context::AppContext;
use anyhow::Result;
use gradience_core::ows::adapter::OwsAdapter;
use std::io::{self, Write};

pub async fn login(ctx: &AppContext) -> Result<()> {
    print!("Enter vault passphrase: ");
    io::stdout().flush()?;
    let mut passphrase = String::new();
    io::stdin().read_line(&mut passphrase)?;
    let passphrase = passphrase.trim();
    if passphrase.len() < 12 {
        anyhow::bail!("Passphrase must be at least 12 characters");
    }
    let vault = ctx.ows.init_vault(passphrase).await?;
    drop(vault);
    ctx.write_passphrase(passphrase)?;
    println!("Vault unlocked successfully. Session saved.");
    Ok(())
}
