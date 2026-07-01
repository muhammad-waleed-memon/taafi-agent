// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

// NOTE: Keep in sync with taafi-agent/src/models.rs and taafi-orchestrator/src/models.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentType {
    Deadlock,
    SlowQuery,
    HighMemory,
    ReplicationLag,
    ConnectionExhaustion,
    HighCpu,
    DiskPressure,
    Unknown,
}

impl std::fmt::Display for IncidentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub incident_type: IncidentType,
    pub severity: Severity,
    pub description: String,
    pub affected_tables: Vec<String>,
    pub query: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixType {
    CreateIndex,
    Analyze,
    Vacuum,
    Reindex,
    TerminateBackend,
    ConfigChange,
    KillOp,
    Custom,
}

impl std::fmt::Display for FixType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub fix_type: FixType,
    pub description: String,
    pub sql: Option<String>,
    pub command: Option<String>,
    pub risk_score: f64,
    pub confidence: f64,
    pub rollback_plan: Option<String>,
    pub approved: bool,
    pub applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    pub fix_id: Uuid,
    pub success: bool,
    pub message: String,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunResult {
    pub success: bool,
    pub estimated_time_ms: u64,
    pub warnings: Vec<String>,
    pub affected_rows: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub status: String,
    pub uptime_secs: u64,
    pub db_type: String,
    pub connected: bool,
    pub incident_count: u64,
    pub fix_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMatch {
    pub pattern_hash: String,
    pub fix_type: FixType,
    pub fix_sql: Option<String>,
    pub confidence: f64,
    pub success_rate: f64,
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum PluginError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Query failed: {0}")]
    QueryFailed(String),
    #[error("Fix failed: {0}")]
    FixFailed(String),
    #[error("Timeout: {0}")]
    Timeout(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum CloudError {
    #[error("Metadata fetch failed: {0}")]
    MetadataFetchFailed(String),
    #[error("Region not compliant: {0}")]
    RegionNotCompliant(String),
    #[error("Network error: {0}")]
    NetworkError(String),
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum CryptoError {
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Encapsulation failed: {0}")]
    EncapsulationFailed(String),
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum LLMError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Parse failed: {0}")]
    ParseFailed(String),
    #[error("Budget exceeded")]
    BudgetExceeded,
    #[error("Timeout: {0}")]
    Timeout(String),
}
