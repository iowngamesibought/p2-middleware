//! Narrow network helper: privileged operations require verified local IPC and elevation.
#[cfg(windows)]
fn main() {
    use p2_middleware::control;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 || args[0] != "--pipe" || args[2] != "--parent" {
        std::process::exit(1);
    }
    let parent = args[3].parse::<u32>().unwrap_or(0);
    let result = control::runtime().and_then(|runtime| {
        runtime.block_on(async {
            let mut pipe = control::client(&args[1], parent)?;
            let result = serve(&mut pipe).await;
            if let Err(reason) = result {
                // A closed/full reporting channel must not abort recovery.
                let _ = control::send(&mut pipe, &serde_json::json!({"event":reason})).await;
            }
            Ok(result.is_ok())
        })
    });
    if !matches!(result, Ok(true)) {
        std::process::exit(1);
    }
}

#[cfg(windows)]
async fn serve(
    pipe: &mut tokio::net::windows::named_pipe::NamedPipeClient,
) -> Result<(), &'static str> {
    use p2_middleware::{
        control::{Command, Lines},
        native_gateway::NativeHost,
        windows_host::{self, JournalStore},
    };
    use std::time::Duration;
    let mut lines = Lines::default();
    let line = tokio::time::timeout(Duration::from_secs(30), lines.next(pipe))
        .await
        .map_err(|_| "request_timeout")?
        .map_err(|_| "invalid_request")?
        .ok_or("request_cancelled")?;
    let command: Command = serde_json::from_slice(&line).map_err(|_| "invalid_request")?;
    if !matches!(command, Command::Session) {
        return Err("invalid_request");
    }
    if !windows_host::elevated() {
        return Err("administrator_required");
    }
    let store = JournalStore::open().map_err(|_| "store_unavailable")?;
    let mut native = NativeHost::new(store);
    p2_middleware::session::run(pipe, lines, &mut native).await
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Windows only");
    std::process::exit(1);
}
