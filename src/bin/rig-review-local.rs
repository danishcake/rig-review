//! A tool for simple but automated code review
//!
//! Reviews can be performed against a source of diffs, and the output written to a local file, or
//! remote gitlab MR.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rig_review::source::local_git::LocalGitReviewSource;

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    let source = LocalGitReviewSource::new(args.input, &args.base, &args.modified)
        .context("Failed to initialise source")?;


    Ok(())
}
