use crate::context::AppContext;
use anyhow::Result;
use gradience_core::ows::adapter::OwsAdapter;
use std::io::{self, Write};

fn api_base() -> String {
    std::env::var("GRADIENCE_API_URL").unwrap_or_else(|_| "http://localhost:8080".into())
}

fn token_path(ctx: &AppContext) -> std::path::PathBuf {
    ctx.data_dir.join(".cli_token")
}

fn read_token(ctx: &AppContext) -> Option<String> {
    std::fs::read_to_string(token_path(ctx)).ok().map(|s| s.trim().to_string())
}

fn write_token(ctx: &AppContext, token: &str) -> Result<()> {
    let path = token_path(ctx);
    std::fs::write(&path, token)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(&path, perms);
    }
    Ok(())
}

pub async fn login(ctx: &AppContext) -> Result<()> {
    let base = api_base();
    let client = reqwest::Client::new();

    let resp: serde_json::Value = client
        .post(format!("{}/api/auth/device/initiate", base))
        .send()
        .await?
        .json()
        .await?;

    let device_code = resp["device_code"].as_str().unwrap_or("");
    let user_code = resp["user_code"].as_str().unwrap_or("");
    let url = resp["verification_url"].as_str().unwrap_or("");

    if device_code.is_empty() || user_code.is_empty() {
        anyhow::bail!("Invalid device auth response from server");
    }

    println!("\nOpen this URL in your browser to authorize this device:");
    println!("  {}", url);

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", url]).spawn();
    }

    println!("\nWaiting for authorization (user code: {})...", user_code);

    for i in 0..100 {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        if i % 10 == 0 {
            print!(".");
            let _ = Write::flush(&mut io::stdout());
        }

        let poll: serde_json::Value = client
            .post(format!("{}/api/auth/device/poll", base))
            .json(&serde_json::json!({ "device_code": device_code }))
            .send()
            .await?
            .json()
            .await?;

        if poll["authorized"].as_bool() == Some(true) {
            if let Some(token) = poll["token"].as_str() {
                write_token(ctx, token)?;
                println!("\nLogin successful! Token saved.");
                return Ok(());
            }
        }
    }

    anyhow::bail!("\nAuthorization timed out. Please try again.")
}

pub async fn whoami(ctx: &AppContext) -> Result<()> {
    let base = api_base();
    let mut remote_ok = false;

    if let Some(token) = read_token(ctx) {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/api/wallets", base))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if res.status().is_success() {
            let wallets: Vec<serde_json::Value> = res.json().await?;
            println!("Remote API: {} (authenticated)", base);
            println!("Wallets linked: {}", wallets.len());
            remote_ok = true;
        } else {
            println!("Remote API token exists but is invalid or expired.");
        }
    } else {
        println!("No remote API token found. Run `gradience auth login` to connect.");
    }

    if let Some(pp) = ctx.read_passphrase() {
        let vault = ctx.ows.init_vault(&pp).await?;
        drop(vault);
        println!("Local vault: unlocked ({})", ctx.vault_dir.display());
    } else {
        println!("Local vault: locked (no passphrase saved)");
    }

    if !remote_ok && ctx.read_passphrase().is_none() {
        anyhow::bail!("Not authenticated with remote API and local vault is locked.");
    }

    Ok(())
}

pub async fn local_unlock(ctx: &AppContext) -> Result<()> {
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
