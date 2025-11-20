use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod app;
mod buck;
mod events;
mod scheduler;
mod ui;
use app::App;
use tracing::info;

#[derive(Parser)]
#[command(name = "buck-tui")]
#[command(about = "A terminal UI for Buck2")]
struct Args {
    #[arg(short, long, help = "Path to the Buck2 project")]
    path: Option<String>,
}

fn setup_logging() -> Result<tracing_appender::non_blocking::WorkerGuard> {
    // Get standard log directory following XDG Base Directory specification
    // Linux: ~/.local/state/buck-tui/
    // macOS: ~/Library/Application Support/buck-tui/
    // Windows: C:\Users\<user>\AppData\Local\buck-tui\
    let log_dir = dirs::state_dir()
        .or_else(|| dirs::data_local_dir())
        .context("Failed to determine log directory")?
        .join("buck-tui");

    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

    // Set up file logging with daily rotation
    let file_appender = tracing_appender::rolling::daily(&log_dir, "buck-tui.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer().with_writer(non_blocking).with_ansi(false), // Disable ANSI colors in log files
        )
        .init();

    info!("Starting buck-tui");
    info!("Log directory: {:?}", log_dir);

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = setup_logging()?;

    let args = Args::parse();
    let project_path = args.path.unwrap_or_else(|| ".".to_string());

    let mut app = App::new(project_path).await?;

    // Request targets for the initial current directory if it has Buck files
    app.initialize().await;

    app.run().await?;

    Ok(())
}
