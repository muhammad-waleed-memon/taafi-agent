// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{Fix, DryRunResult, PluginError};
use crate::plugin_manager::Plugin;
use tracing::info;

pub struct SandboxEngine;

impl SandboxEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn dry_run(&self, fix: &Fix, plugin: &dyn Plugin) -> Result<DryRunResult, PluginError> {
        info!("Executing sandbox dry run for fix {}", fix.id);
        plugin.dry_run(fix).await
    }
}
