//! HTTP server for triggering GitLab merge request reviews from webhooks.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use rig_review::{config::Config, logging::initialise_logging};

#[derive(Debug, Parser)]
struct Args {
    /// The configuration file to use. Configuration values override those picked up from the environment.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Increase verbosity (-v, -vv, -vvv). Defaults to INFO
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Decrease verbosity (-q, -qq). Defaults to INFO
    #[arg(short, action = clap::ArgAction::Count)]
    quiet: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    initialise_logging(args.verbose, args.quiet);

    let config = Config::new(&args.config)?;
    rig_review::server::run_server(config).await?;

    Ok(())
}
