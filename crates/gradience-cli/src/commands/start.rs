pub async fn run() -> anyhow::Result<()> {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let open_url = format!(
        "http://{}/login",
        bind_addr.replace("0.0.0.0", "127.0.0.1").replace("[::]", "127.0.0.1")
    );

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&open_url).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(&open_url).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd").args(&["/C", "start", &open_url]).spawn();
    });

    gradience_api::run().await
}
