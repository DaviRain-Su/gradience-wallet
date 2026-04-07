mod args;
mod mcp;
mod server;
mod tools;

fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = server::run_stdio_server() {
        eprintln!("gradience-mcp error: {}", e);
        std::process::exit(1);
    }
}
