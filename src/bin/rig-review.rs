//! A tool for simple but automated code review
//!
//! Reviews can be performed against a source of diffs, and the output written to a local file, or
//! remote gitlab MR.

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rig_review::{
    config::Config,
    logging::initialise_logging,
    review::{base::ReviewEngine, rig::engine::RigReviewEngine},
    sink::{base::ReviewSink, local_stdout::local_stdout_sink::LocalStdoutSink},
    source::{
        base::ReviewSource, gitlab::gitlab_review_source::GitlabReviewSource,
        local::local_git_review_source::LocalGitReviewSource,
    },
};
use tokio::sync::Mutex;

#[derive(Debug, Parser)]
struct Args {
    /// The configuration file to use. Configuration values override those pick up from the environment
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Ignore invalid certicicates for all network traffic
    #[arg[short, long, default_value_t = false]]
    ignore_certs: bool,

    /// The source is set using a subcommand
    #[command(subcommand)]
    source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SinkForGitlabSource {
    Gitlab,
    Stdout,
}

#[derive(Debug, Subcommand)]
enum Source {
    /// A local git repository.
    /// Output will be written to stdouts
    Local {
        /// A folder containing a git repository
        #[arg(short, long)]
        input: PathBuf,

        /// The base commit hash, tag or branch
        #[arg(short, long)]
        base: String,

        /// The modified commit hash, tag or branch
        #[arg(short, long)]
        modified: String,
    },

    /// A remote gitlab instance.
    /// Output will output as comments on the merge request
    Gitlab {
        /// The URL for a merge request
        #[arg(short, long)]
        url: String,

        #[arg(short, long, default_value = "gitlab")]
        sink: SinkForGitlabSource,
    },
    // TODO: Folders
    // TODO: Arbitrary git remotes
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args
    let args = Args::parse();

    // Initialise logging
    initialise_logging(args.verbose);

    // Load config
    let mut config = Config::new(&args.config)?;

    // Merge CLI arguments into config
    if args.ignore_certs {
        config.network.ignore_certs = args.ignore_certs;
    }

    // Create the source and sink for the review
    let (source, sink) = match args.source {
        Source::Local {
            input,
            base,
            modified,
        } => {
            let source: Arc<Mutex<dyn ReviewSource>> = Arc::new(Mutex::new(
                LocalGitReviewSource::new(input, &base, &modified)
                    .context("Failed to initialise source")?,
            ));
            let sink: Arc<Mutex<dyn ReviewSink>> = Arc::new(Mutex::new(LocalStdoutSink::new()));
            (source, sink)
        }
        Source::Gitlab { url, sink } => {
            let gitlab_config = config.gitlab.context("Gitlab configuration missing")?;
            let source = GitlabReviewSource::new(&url, &gitlab_config, &config.network).await?;
            let sink: Arc<Mutex<dyn ReviewSink>> = match sink {
                SinkForGitlabSource::Gitlab => Arc::new(Mutex::new(source.create_sink())),
                SinkForGitlabSource::Stdout => Arc::new(Mutex::new(LocalStdoutSink::new())),
            };

            let source: Arc<Mutex<dyn ReviewSource>> = Arc::new(Mutex::new(source));

            (source, sink)
        }
    };

    // Create review engine
    let mut review_engine = RigReviewEngine::new(&config.rig_review_engine, &config.network);

    // Perform the review
    review_engine.review(source, sink).await?;

    Ok(())
}
