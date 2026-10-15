use tracing::{Level, level_filters::LevelFilter};
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};

pub fn initialise_logging(verbose: u8) {
    let max_level = match verbose {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };

    // Setup logging
    let subscriber = tracing_subscriber::fmt::layer()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false)
        // Write to stderr
        .with_writer(std::io::stderr);

    let filter = Targets::new()
        // Log errors from dependencies
        .with_default(LevelFilter::WARN)
        // Log more from this crate
        .with_target(env!("CARGO_CRATE_NAME"), max_level);

    tracing_subscriber::registry()
        .with(filter)
        .with(subscriber)
        .init();
}

/// Determines if a tracing level is enabled. You cannot directly use tracing::enabled
/// as the macro requires a constant
pub fn is_level_enabled(level: &Level) -> bool {
    match *level {
        Level::ERROR => tracing::enabled!(Level::ERROR),
        Level::WARN => tracing::enabled!(Level::WARN),
        Level::INFO => tracing::enabled!(Level::INFO),
        Level::DEBUG => tracing::enabled!(Level::DEBUG),
        Level::TRACE => tracing::enabled!(Level::TRACE),
    }
}
