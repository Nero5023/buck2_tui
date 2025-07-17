use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod buck;
mod events;
mod scheduler;
mod ui;
use tracing::info;

use app::App;

#[derive(Parser)]
#[command(name = "buck-tui")]
#[command(about = "A terminal UI for Buck2")]
struct Args {
    #[arg(short, long, help = "Path to the Buck2 project")]
    path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;

    // Set up file logging
    let file_appender = tracing_appender::rolling::daily("logs", "buck-tui.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer().with_writer(non_blocking).with_ansi(false), // Disable ANSI colors in log files
        )
        .init();

    info!("Starting buck-tui");

    let args = Args::parse();
    let project_path = args.path.unwrap_or_else(|| ".".to_string());

    let mut app = App::new(project_path).await?;
    
    // Request targets for the initial current directory if it has Buck files
    app.initialize().await;
    
    app.run().await?;

    Ok(())
}
