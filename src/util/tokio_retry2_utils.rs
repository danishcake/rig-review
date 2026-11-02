use std::{sync::Arc, time::Duration};

/// A type that can be used to store a retry strategy
pub type RetryStrategy = Box<dyn Iterator<Item = Duration> + Send>;

/// A factory that creates retry strategies - these are consumed, so much be recreated
/// for each use
pub type RetryStrategyFactory = Arc<dyn Fn() -> RetryStrategy + Send + Sync>;
