// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{Fix, FixResult, PluginError};
use crate::plugin_manager::Plugin;
use regex::Regex;
use std::time::Instant;
use tracing::{info, warn, error};

pub struct FixExecutor {
    whitelist: Vec<Regex>,
}

impl FixExecutor {
    pub fn new() -> Self {
        let patterns = vec![
            r"(?i)^\s*CREATE\s+(UNIQUE\s+)?INDEX\s+",
            r"(?i)^\s*ANALYZE\s+",
            r"(?i)^\s*VACUUM\s+",
            r"(?i)^\s*REINDEX\s+",
            r"(?i)^\s*SELECT\s+pg_terminate_backend\s*\(",
            r"(?i)^\s*CLIENT\s+KILL\s+",
            r"(?i)^\s*CONFIG\s+SET\s+slowlog-log-slower-than\s+",
            r"(?i)^\s*db\.killOp\s*\(",
            r"(?i)^\s*db\..+\.createIndex\s*\(",
        ];

        let whitelist = patterns.into_iter()
            .map(|p| Regex::new(p).expect("Invalid regex in whitelist"))
            .collect();

        Self { whitelist }
    }

    pub fn validate_fix(&self, fix: &Fix) -> Result<(), PluginError> {
        let sql_or_cmd = if let Some(ref sql) = fix.sql {
            sql.as_str()
        } else if let Some(ref cmd) = fix.command {
            cmd.as_str()
        } else {
            return Err(PluginError::FixFailed("No SQL or command specified in fix".to_string()));
        };

        let is_whitelisted = self.whitelist.iter().any(|re| re.is_match(sql_or_cmd));

        if !is_whitelisted {
            error!("Security Alert: Fix SQL or command violates safety whitelist: {}", sql_or_cmd);
            return Err(PluginError::FixFailed("Security Alert: Execution blocked. Operation is not on SQL whitelist.".to_string()));
        }

        Ok(())
    }

    pub async fn apply_fix(&self, fix: &Fix, plugin: &dyn Plugin) -> Result<FixResult, PluginError> {
        self.validate_fix(fix)?;

        info!("Applying fix {}: {}", fix.id, fix.description);
        let start = Instant::now();

        let result = plugin.apply_fix(fix).await;
        let elapsed = start.elapsed().as_millis() as u64;

        match result {
            Ok(res) => {
                info!("Fix {} applied successfully in {} ms", fix.id, elapsed);
                Ok(FixResult {
                    fix_id: fix.id,
                    success: res.success,
                    message: res.message,
                    execution_time_ms: elapsed,
                })
            }
            Err(e) => {
                error!("Fix {} failed: {:?}", fix.id, e);
                Err(e)
            }
        }
    }

    pub async fn rollback(&self, fix: &Fix, plugin: &dyn Plugin) -> Result<FixResult, PluginError> {
        let rollback_plan = match fix.rollback_plan {
            Some(ref plan) if !plan.trim().is_empty() => plan,
            _ => {
                warn!("No rollback plan available for fix {}", fix.id);
                return Ok(FixResult {
                    fix_id: fix.id,
                    success: false,
                    message: "No rollback plan available".to_string(),
                    execution_time_ms: 0,
                });
            }
        };

        // Validate rollback sql
        let dummy_rollback_fix = Fix {
            id: fix.id,
            incident_id: fix.incident_id,
            fix_type: fix.fix_type,
            description: format!("Rollback of {}", fix.id),
            sql: Some(rollback_plan.clone()),
            command: None,
            risk_score: 0.0,
            confidence: 1.0,
            rollback_plan: None,
            approved: true,
            applied: false,
        };

        self.validate_fix(&dummy_rollback_fix)?;

        info!("Executing rollback for fix {}: {}", fix.id, rollback_plan);
        let start = Instant::now();
        let result = plugin.apply_fix(&dummy_rollback_fix).await;
        let elapsed = start.elapsed().as_millis() as u64;

        match result {
            Ok(res) => {
                info!("Rollback for fix {} executed successfully", fix.id);
                Ok(FixResult {
                    fix_id: fix.id,
                    success: res.success,
                    message: format!("Rollback: {}", res.message),
                    execution_time_ms: elapsed,
                })
            }
            Err(e) => {
                error!("Rollback for fix {} failed: {:?}", fix.id, e);
                Err(e)
            }
        }
    }
}
