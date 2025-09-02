use std::{env, str::FromStr};

use anyhow::Result;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn initialize_logger() -> Result<()> {
    let log_level = env::var("LOG_LEVEL").unwrap_or("info".to_string());
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true)
                .with_target(true),
        )
        .with(LevelFilter::from_str(&log_level)?)
        .init();
    return Ok(());
}
