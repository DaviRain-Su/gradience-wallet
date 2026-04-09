use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub async fn run() -> anyhow::Result<()> {
    let repo_root = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default())
        .join("../../..");
    let repo_root = repo_root.canonicalize().unwrap_or(repo_root);

    let api_dir = repo_root.clone();
    let web_dir = repo_root.join("web");

    println!("[Gradience] Starting local API server...");
    let mut api_child = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("gradience-api")
        .current_dir(&api_dir)
        .env("DATABASE_URL", "sqlite:./gradience.db?mode=rwc")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn API server: {}", e))?;

    println!("[Gradience] Starting web frontend...");
    let mut web_child = Command::new("npm")
        .arg("run")
        .arg("dev")
        .current_dir(&web_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn web dev server: {}", e))?;

    println!("[Gradience] Waiting for services to be ready...");
    let mut ready = false;
    for _ in 0..60 {
        if check_ready().await {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }

    if ready {
        println!("[Gradience] Ready! Open http://localhost:3000 in your browser.");
        let _ = open_browser("http://localhost:3000");
    } else {
        eprintln!("[Gradience] Timed out waiting for services. Check logs above.");
    }

    // Block until either child exits
    loop {
        match api_child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("[Gradience] API server exited with {}", status);
                let _ = web_child.kill();
                break;
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("[Gradience] Error checking API server: {}", e);
                let _ = web_child.kill();
                break;
            }
        }
        match web_child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("[Gradience] Web server exited with {}", status);
                let _ = api_child.kill();
                break;
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("[Gradience] Error checking web server: {}", e);
                let _ = api_child.kill();
                break;
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

async fn check_ready() -> bool {
    let web_ok = reqwest::get("http://localhost:3000")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    let api_ok = reqwest::get("http://localhost:8080/health")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    web_ok && api_ok
}

fn open_browser(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", url]).spawn()?;
    }
    Ok(())
}
