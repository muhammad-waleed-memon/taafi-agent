// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use std::env;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Config {
    pub agent_id: String,
    pub orchestrator_url: String,
    pub database_url: String,
    pub pq_security_level: String,
    pub memory_db_path: String,
    pub heartbeat_interval_secs: u64,
    pub metrics_interval_secs: u64,
    pub rust_log: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let agent_id = env::var("AGENT_ID")
            .unwrap_or_else(|_| Uuid::new_v4().to_string());
        
        let orchestrator_url = env::var("ORCHESTRATOR_URL")
            .unwrap_or_else(|_| "http://localhost:50051".to_string());

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://taafi_agent.db".to_string());

        let pq_security_level = env::var("PQ_SECURITY_LEVEL")
            .unwrap_or_else(|_| "Level3".to_string());

        let memory_db_path = env::var("MEMORY_DB_PATH")
            .unwrap_or_else(|_| "taafi_memory.db".to_string());

        let heartbeat_interval_secs = env::var("HEARTBEAT_INTERVAL_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|e| format!("Invalid HEARTBEAT_INTERVAL_SECS: {}", e))?;

        let metrics_interval_secs = env::var("METRICS_INTERVAL_SECS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u64>()
            .map_err(|e| format!("Invalid METRICS_INTERVAL_SECS: {}", e))?;

        let rust_log = env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string());

        Ok(Config {
            agent_id,
            orchestrator_url,
            database_url,
            pq_security_level,
            memory_db_path,
            heartbeat_interval_secs,
            metrics_interval_secs,
            rust_log,
        })
    }
}
