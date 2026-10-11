//! A tool for simple but automated code review
//!
//! Reviews can be performed against a source of diffs, and the output written to a local file, or
//! remote gitlab MR.

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use rig_review::{
    config::Config,
    logging::initialise_logging,
    review::{base::ReviewEngine, rig::engine::RigReviewEngine},
    sink::local_stdout::LocalStdoutSink,
    source::local_git::LocalGitReviewSource,
};
use tokio::sync::Mutex;

#[derive(Debug, Parser)]
struct Args {
    /// A folder containing a git repository
    #[arg(short, long)]
    input: PathBuf,

    /// The base commit hash, tag or branch
    #[arg(short, long)]
    base: String,

    /// The modified commit hash, tag or branch
    #[arg(short, long)]
    modified: String,

    /// The configuration file to use. Configuration values override those pick up from the environment
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args
    let args = Args::parse();

    // Initialise logging
    initialise_logging(args.verbose);

    // Load config
    let config = Config::new(&args.config)?;

    // Create the source and sink for the review
    let source = Arc::new(Mutex::new(
        LocalGitReviewSource::new(args.input, &args.base, &args.modified)
            .context("Failed to initialise source")?,
    ));
    let sink = Arc::new(Mutex::new(LocalStdoutSink::new()));

    // Create review engine
    let mut review_engine = RigReviewEngine::new(&config.rig_review_engine);

    // Perform the review
    review_engine.review(source, sink).await?;

    Ok(())
}
