#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "weas=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let args: Vec<String> = std::env::args().collect();

    // If explicit CLI arguments are passed (e.g. `weas run`, `weas sub add`), run in CLI mode
    if args.len() > 1 && args[1] != "--gui" {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            weas::run_cli().await;
        });
        return Ok(());
    }

    // Default: launch modern desktop GUI window without console!
    weas::gui::run_gui()
}
