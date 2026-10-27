//! A tool for simple but automated code review
//!
//! Reviews can be performed against a source of diffs, and the output written to a local file, or
//! remote gitlab MR.

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rig_review::{
    config::Config,
    error::RigReviewError,
    logging::initialise_logging,
    review::{base::ReviewEngine, rig::engine::RigReviewEngine},
    sink::{
        base::ReviewSink, discord::discord_sink::DiscordSink,
        local_stdout::local_stdout_sink::LocalStdoutSink,
    },
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

    /// Additional certificates to trust
    #[arg[short, long, action = clap::ArgAction::Append]]
    extra_ca: Vec<PathBuf>,

    /// The source is set using a subcommand
    #[command(subcommand)]
    source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SinkForGitlabSource {
    Gitlab,
    Stdout,
    Discord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SinkForLocalSource {
    Stdout,
    Discord,
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

        #[arg(short, long, value_enum, default_values_t = [SinkForLocalSource::Stdout])]
        sinks: Vec<SinkForLocalSource>,
    },

    /// A remote gitlab instance.
    /// Output will output as comments on the merge request
    Gitlab {
        /// The URL for a merge request
        #[arg(short, long)]
        url: String,

        #[arg(short, long, value_enum, default_values_t = [SinkForGitlabSource::Gitlab])]
        sinks: Vec<SinkForGitlabSource>,
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

    // Create the source and sinks for the review
    let (source, sinks) = match args.source {
        Source::Local {
            input,
            base,
            modified,
            sinks,
        } => {
            let source: Arc<Mutex<dyn ReviewSource>> = Arc::new(Mutex::new(
                LocalGitReviewSource::new(input, &base, &modified)
                    .context("Failed to initialise source")?,
            ));
            let sinks = sinks
                .iter()
                .map(|sink| match sink {
                    SinkForLocalSource::Stdout => {
                        let sink: Arc<Mutex<dyn ReviewSink>> =
                            Arc::new(Mutex::new(LocalStdoutSink::new()));
                        Ok(sink)
                    }
                    SinkForLocalSource::Discord => {
                        let sink: Arc<Mutex<dyn ReviewSink>> =
                            Arc::new(Mutex::new(DiscordSink::new(&config.discord)?));
                        Ok(sink)
                    }
                })
                .collect::<Result<Vec<Arc<Mutex<dyn ReviewSink>>>, RigReviewError>>()?;
            (source, sinks)
        }
        Source::Gitlab { url, sinks } => {
            let gitlab_config = config.gitlab.context("Gitlab configuration missing")?;
            let source = GitlabReviewSource::new(&url, &gitlab_config, &config.network).await?;
            let sinks = sinks
                .iter()
                .map(|sink| match sink {
                    SinkForGitlabSource::Gitlab => {
                        let sink: Arc<Mutex<dyn ReviewSink>> =
                            Arc::new(Mutex::new(source.create_sink()));
                        Ok(sink)
                    }
                    SinkForGitlabSource::Stdout => {
                        let sink: Arc<Mutex<dyn ReviewSink>> =
                            Arc::new(Mutex::new(LocalStdoutSink::new()));
                        Ok(sink)
                    }
                    SinkForGitlabSource::Discord => {
                        let sink: Arc<Mutex<dyn ReviewSink>> =
                            Arc::new(Mutex::new(DiscordSink::new(&config.discord)?));
                        Ok(sink)
                    }
                })
                .collect::<Result<Vec<Arc<Mutex<dyn ReviewSink>>>, RigReviewError>>()?;

            let source: Arc<Mutex<dyn ReviewSource>> = Arc::new(Mutex::new(source));

            (source, sinks)
        }
    };

    // Create review engine
    let mut review_engine = RigReviewEngine::new(&config.rig_review_engine, &config.network);

    // Perform the review
    review_engine.review(source, sinks).await?;

    Ok(())
}
