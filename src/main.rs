use anyhow::Result;
use clap::Parser;
use tracing_subscriber;

mod app;
mod buck;
mod ui;
mod events;

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
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let project_path = args.path.unwrap_or_else(|| ".".to_string());

    let mut app = App::new(project_path).await?;
    app.run().await?;

    Ok(())
}
