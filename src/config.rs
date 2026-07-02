// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use std::env;
use std::fs;
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
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let agent_id = env::var("AGENT_ID")
            .unwrap_or_else(|_| Uuid::new_v4().to_string());
        
        let orchestrator_url = env::var("ORCHESTRATOR_URL")
            .unwrap_or_else(|_| "https://localhost:50051".to_string());

        // Resolve database url - check file option first for production container security
        let database_url = if let Ok(path) = env::var("DATABASE_URL_FILE") {
            fs::read_to_string(&path)
                .map(|s| s.trim().to_string())
                .map_err(|e| format!("Failed to read DATABASE_URL_FILE at {}: {}", path, e))?
        } else {
            env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL environment variable is required and has no default".to_string())?
        };

        // Format Validation: database URL must have a valid scheme and not be sqlite in production
        if database_url.is_empty() {
            return Err("DATABASE_URL cannot be empty".to_string());
        }
        if env::var("TAAFI_ENV").unwrap_or_default() == "production" && database_url.starts_with("sqlite:") {
            return Err("SQLite database is forbidden in production environments (PCI-DSS compliance)".to_string());
        }

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

        let cert_path = env::var("CERT_PATH")
            .unwrap_or_else(|_| "/certs/agent.crt".to_string());

        let key_path = env::var("KEY_PATH")
            .unwrap_or_else(|_| "/certs/agent.key".to_string());

        let ca_path = env::var("CA_PATH")
            .unwrap_or_else(|_| "/certs/ca.crt".to_string());

        Ok(Config {
            agent_id,
            orchestrator_url,
            database_url,
            pq_security_level,
            memory_db_path,
            heartbeat_interval_secs,
            metrics_interval_secs,
            rust_log,
            cert_path,
            key_path,
            ca_path,
        })
    }
}


