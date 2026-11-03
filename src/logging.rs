use tracing::Level;
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};

pub fn initialise_logging(verbose: u8, quiet: u8) {
    let (dep_level, crate_level) = match 2u8.saturating_add(verbose).saturating_sub(quiet) {
        0 => (Level::TRACE, Level::TRACE),
        1 => (Level::ERROR, Level::ERROR),
        2 => (Level::WARN, Level::WARN),
        3 => (Level::WARN, Level::INFO),
        4 => (Level::WARN, Level::DEBUG),
        5 => (Level::WARN, Level::TRACE),
        6 => (Level::INFO, Level::TRACE),
        7 => (Level::DEBUG, Level::TRACE),
        _ => (Level::TRACE, Level::TRACE),
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
        .with_default(dep_level)
        // Log more from this crate
        .with_target(env!("CARGO_CRATE_NAME"), crate_level);

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
