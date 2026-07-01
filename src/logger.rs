// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use std::env;

pub fn init_logger() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let log_format = env::var("TAAFI_LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());
    let filter = EnvFilter::from_default_env();

    if log_format == "json" {
        tracing_subscriber::registry()
            .with(fmt::layer().json().with_current_span(false))
            .with(filter)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(fmt::layer().pretty())
            .with(filter)
            .init();
    }
    
    Ok(())
}
