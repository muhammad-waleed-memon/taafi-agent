// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{Metric, Incident, Fix, DryRunResult, PluginError, Severity, IncidentType, FixType};
use crate::plugin_manager::{Plugin, ClonePlugin};
use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

// --- UNIVERSAL PLUGIN WRAPPER ---

#[derive(Clone)]
pub struct UniversalPluginWrapper {
    pub inner: taafi_plugin_universal::UniversalPlugin,
}

#[async_trait]
impl Plugin for UniversalPluginWrapper {
    fn name(&self) -> &'static str { "universal" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn supported_backends(&self) -> Vec<&'static str> { vec!["universal"] }

    async fn connect(&self, connection_string: &str) -> Result<(), PluginError> {
        let mut mut_self = self.clone();
        mut_self.inner.connect(connection_string).await
            .map_err(|e| PluginError::ConnectionFailed(e))
    }

    async fn disconnect(&self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        self.inner.is_healthy().await
    }

    async fn collect_metrics(&self) -> Vec<Metric> {
        Vec::new()
    }

    async fn detect_incidents(&self) -> Vec<Incident> {
        Vec::new()
    }

    async fn suggest_fixes(&self, _incident: &Incident) -> Vec<Fix> {
        Vec::new()
    }

    async fn apply_fix(&self, _fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }

    async fn dry_run(&self, _fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }
}

// --- POSTGRESQL PLUGIN WRAPPER ---

#[derive(Clone)]
pub struct PostgreSQLPluginWrapper {
    pub inner: taafi_plugin_postgresql::PostgreSQLPlugin,
}

#[async_trait]
impl Plugin for PostgreSQLPluginWrapper {
    fn name(&self) -> &'static str { "postgresql" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn supported_backends(&self) -> Vec<&'static str> { vec!["postgresql", "postgres"] }

    async fn connect(&self, connection_string: &str) -> Result<(), PluginError> {
        self.inner.connect(connection_string).await
            .map_err(|e| PluginError::ConnectionFailed(e))
    }

    async fn disconnect(&self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        self.inner.is_healthy().await
    }

    async fn collect_metrics(&self) -> Vec<Metric> {
        let mut metrics = Vec::new();
        let timestamp = Utc::now();

        // Query active connections count
        if let Ok(rows) = self.inner.query_rows("SELECT count(*) FROM pg_stat_activity").await {
            if let Some(row) = rows.first() {
                let count: i64 = row.get(0);
                metrics.push(Metric {
                    name: "db.postgresql.connections.active".to_string(),
                    value: count as f64,
                    labels: HashMap::new(),
                    timestamp,
                });
            }
        }
        
        metrics
    }

    async fn detect_incidents(&self) -> Vec<Incident> {
        let mut incidents = Vec::new();

        // Deadlock detection query
        let deadlock_sql = "
            SELECT 
                blocked_locks.pid     AS blocked_pid,
                blocked_activity.usename  AS blocked_user,
                blocking_locks.pid    AS blocking_pid,
                blocking_activity.usename AS blocking_user,
                blocked_activity.query    AS blocked_statement,
                blocking_activity.query   AS blocking_statement
            FROM  pg_catalog.pg_locks         blocked_locks
            JOIN pg_catalog.pg_stat_activity blocked_activity ON blocked_activity.pid = blocked_locks.pid
            JOIN pg_catalog.pg_locks         blocking_locks 
                ON blocking_locks.locktype = blocked_locks.locktype
                AND blocking_locks.database IS NOT DISTINCT FROM blocked_locks.database
                AND blocking_locks.relation IS NOT DISTINCT FROM blocked_locks.relation
                AND blocking_locks.page IS NOT DISTINCT FROM blocked_locks.page
                AND blocking_locks.tuple IS NOT DISTINCT FROM blocked_locks.tuple
                AND blocking_locks.virtualxid IS NOT DISTINCT FROM blocked_locks.virtualxid
                AND blocking_locks.transactionid IS NOT DISTINCT FROM blocked_locks.transactionid
                AND blocking_locks.classid IS NOT DISTINCT FROM blocked_locks.classid
                AND blocking_locks.objid IS NOT DISTINCT FROM blocked_locks.objid
                AND blocking_locks.objsubid IS NOT DISTINCT FROM blocked_locks.objsubid
                AND blocking_locks.pid != blocked_locks.pid
            JOIN pg_catalog.pg_stat_activity blocking_activity ON blocking_activity.pid = blocking_locks.pid
            WHERE NOT blocked_locks.granted;
        ";

        if let Ok(rows) = self.inner.query_rows(deadlock_sql).await {
            for r in rows {
                let blocked_pid: i32 = r.get("blocked_pid");
                let blocking_pid: i32 = r.get("blocking_pid");
                let blocked_query: String = r.get("blocked_statement");
                
                incidents.push(Incident {
                    id: Uuid::new_v4(),
                    incident_type: IncidentType::Deadlock,
                    severity: Severity::Critical,
                    description: format!("Process PID {} is blocked by PID {} due to a lock contention.", blocked_pid, blocking_pid),
                    affected_tables: Vec::new(),
                    query: Some(blocked_query),
                    timestamp: Utc::now(),
                    agent_id: String::new(),
                    resolved: false,
                });
            }
        }

        // Slow query detection
        let slow_sql = "SELECT pid, query, state, now() - query_start as duration FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '5 seconds'";
        if let Ok(rows) = self.inner.query_rows(slow_sql).await {
            for r in rows {
                let pid: i32 = r.get("pid");
                let query: String = r.get("query");
                incidents.push(Incident {
                    id: Uuid::new_v4(),
                    incident_type: IncidentType::SlowQuery,
                    severity: Severity::High,
                    description: format!("Active slow query running on PID {} for over 5 seconds.", pid),
                    affected_tables: Vec::new(),
                    query: Some(query),
                    timestamp: Utc::now(),
                    agent_id: String::new(),
                    resolved: false,
                });
            }
        }

        incidents
    }

    async fn suggest_fixes(&self, incident: &Incident) -> Vec<Fix> {
        let mut fixes = Vec::new();
        match incident.incident_type {
            IncidentType::Deadlock => {
                // We can terminate the blocking backend
                fixes.push(Fix {
                    id: Uuid::new_v4(),
                    incident_id: incident.id,
                    fix_type: FixType::TerminateBackend,
                    description: "Terminate the blocking backend process to resolve the lock conflict.".to_string(),
                    sql: Some("SELECT pg_terminate_backend(pid);".to_string()),
                    command: None,
                    risk_score: 0.1,
                    confidence: 0.95,
                    rollback_plan: None,
                    approved: false,
                    applied: false,
                });
            }
            IncidentType::SlowQuery => {
                fixes.push(Fix {
                    id: Uuid::new_v4(),
                    incident_id: incident.id,
                    fix_type: FixType::Analyze,
                    description: "Run ANALYZE to update statistics for query optimizer planning.".to_string(),
                    sql: Some("ANALYZE;".to_string()),
                    command: None,
                    risk_score: 0.05,
                    confidence: 0.8,
                    rollback_plan: None,
                    approved: false,
                    applied: false,
                });
            }
            _ => {}
        }
        fixes
    }

    async fn apply_fix(&self, fix: &Fix) -> Result<DryRunResult, PluginError> {
        if let Some(ref sql) = fix.sql {
            self.inner.execute_query(sql).await
                .map(|rows| DryRunResult {
                    success: true,
                    estimated_time_ms: 0,
                    warnings: Vec::new(),
                    affected_rows: Some(rows as i64),
                })
                .map_err(|e| PluginError::FixFailed(e))
        } else {
            Err(PluginError::FixFailed("No SQL command specified".to_string()))
        }
    }

    async fn dry_run(&self, fix: &Fix) -> Result<DryRunResult, PluginError> {
        // Implement transaction-based dry-run (Begin -> Exec -> Rollback)
        if let Some(ref sql) = fix.sql {
            let dry_run_sql = format!("BEGIN; {}; ROLLBACK;", sql);
            self.inner.execute_query(&dry_run_sql).await
                .map(|_| DryRunResult {
                    success: true,
                    estimated_time_ms: 10,
                    warnings: Vec::new(),
                    affected_rows: Some(0),
                })
                .map_err(|e| PluginError::FixFailed(e))
        } else {
            Err(PluginError::FixFailed("No SQL command specified".to_string()))
        }
    }
}

// --- MYSQL PLUGIN WRAPPER ---

#[derive(Clone)]
pub struct MySQLPluginWrapper {
    pub inner: taafi_plugin_mysql::MySQLPlugin,
}

#[async_trait]
impl Plugin for MySQLPluginWrapper {
    fn name(&self) -> &'static str { "mysql" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn supported_backends(&self) -> Vec<&'static str> { vec!["mysql"] }

    async fn connect(&self, connection_string: &str) -> Result<(), PluginError> {
        self.inner.connect(connection_string).await
            .map_err(|e| PluginError::ConnectionFailed(e))
    }

    async fn disconnect(&self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        self.inner.is_healthy().await
    }

    async fn collect_metrics(&self) -> Vec<Metric> {
        Vec::new()
    }

    async fn detect_incidents(&self) -> Vec<Incident> {
        Vec::new()
    }

    async fn suggest_fixes(&self, _incident: &Incident) -> Vec<Fix> {
        Vec::new()
    }

    async fn apply_fix(&self, fix: &Fix) -> Result<DryRunResult, PluginError> {
        if let Some(ref sql) = fix.sql {
            self.inner.execute_query(sql).await
                .map(|rows| DryRunResult {
                    success: true,
                    estimated_time_ms: 0,
                    warnings: Vec::new(),
                    affected_rows: Some(rows as i64),
                })
                .map_err(|e| PluginError::FixFailed(e))
        } else {
            Err(PluginError::FixFailed("No SQL specified".to_string()))
        }
    }

    async fn dry_run(&self, fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }
}

// --- MONGODB PLUGIN WRAPPER ---

#[derive(Clone)]
pub struct MongoDBPluginWrapper {
    pub inner: taafi_plugin_mongodb::MongoDBPlugin,
}

#[async_trait]
impl Plugin for MongoDBPluginWrapper {
    fn name(&self) -> &'static str { "mongodb" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn supported_backends(&self) -> Vec<&'static str> { vec!["mongodb", "mongo"] }

    async fn connect(&self, connection_string: &str) -> Result<(), PluginError> {
        self.inner.connect(connection_string).await
            .map_err(|e| PluginError::ConnectionFailed(e))
    }

    async fn disconnect(&self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        self.inner.is_healthy().await
    }

    async fn collect_metrics(&self) -> Vec<Metric> {
        Vec::new()
    }

    async fn detect_incidents(&self) -> Vec<Incident> {
        Vec::new()
    }

    async fn suggest_fixes(&self, _incident: &Incident) -> Vec<Fix> {
        Vec::new()
    }

    async fn apply_fix(&self, _fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }

    async fn dry_run(&self, _fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }
}

// --- REDIS PLUGIN WRAPPER ---

#[derive(Clone)]
pub struct RedisPluginWrapper {
    pub inner: taafi_plugin_redis::RedisPlugin,
}

#[async_trait]
impl Plugin for RedisPluginWrapper {
    fn name(&self) -> &'static str { "redis" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn supported_backends(&self) -> Vec<&'static str> { vec!["redis", "valkey"] }

    async fn connect(&self, connection_string: &str) -> Result<(), PluginError> {
        self.inner.connect(connection_string).await
            .map_err(|e| PluginError::ConnectionFailed(e))
    }

    async fn disconnect(&self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        self.inner.is_healthy().await
    }

    async fn collect_metrics(&self) -> Vec<Metric> {
        Vec::new()
    }

    async fn detect_incidents(&self) -> Vec<Incident> {
        Vec::new()
    }

    async fn suggest_fixes(&self, _incident: &Incident) -> Vec<Fix> {
        Vec::new()
    }

    async fn apply_fix(&self, fix: &Fix) -> Result<DryRunResult, PluginError> {
        if let Some(ref cmd) = fix.command {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                return Err(PluginError::FixFailed("Empty command".to_string()));
            }
            let cmd_name = parts[0];
            let args = &parts[1..];
            self.inner.execute_cmd::<String>(cmd_name, args).await
                .map(|_| DryRunResult {
                    success: true,
                    estimated_time_ms: 0,
                    warnings: Vec::new(),
                    affected_rows: None,
                })
                .map_err(|e| PluginError::FixFailed(e))
        } else {
            Err(PluginError::FixFailed("No command specified".to_string()))
        }
    }

    async fn dry_run(&self, _fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }
}

// --- KAFKA PLUGIN WRAPPER ---

#[derive(Clone)]
pub struct KafkaPluginWrapper {
    pub inner: taafi_plugin_kafka::KafkaPlugin,
}

#[async_trait]
impl Plugin for KafkaPluginWrapper {
    fn name(&self) -> &'static str { "kafka" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn supported_backends(&self) -> Vec<&'static str> { vec!["kafka"] }

    async fn connect(&self, connection_string: &str) -> Result<(), PluginError> {
        self.inner.connect(connection_string).await
            .map_err(|e| PluginError::ConnectionFailed(e))
    }

    async fn disconnect(&self) -> Result<(), PluginError> {
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        self.inner.is_healthy().await
    }

    async fn collect_metrics(&self) -> Vec<Metric> {
        Vec::new()
    }

    async fn detect_incidents(&self) -> Vec<Incident> {
        Vec::new()
    }

    async fn suggest_fixes(&self, _incident: &Incident) -> Vec<Fix> {
        Vec::new()
    }

    async fn apply_fix(&self, _fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }

    async fn dry_run(&self, _fix: &Fix) -> Result<DryRunResult, PluginError> {
        Ok(DryRunResult { success: true, estimated_time_ms: 0, warnings: Vec::new(), affected_rows: Some(0) })
    }
}

// --- STATIC LOAD FUNCTION ---

pub fn load_plugins() -> Vec<Box<dyn Plugin>> {
    vec![
        Box::new(UniversalPluginWrapper { inner: taafi_plugin_universal::UniversalPlugin::new() }),
        Box::new(PostgreSQLPluginWrapper { inner: taafi_plugin_postgresql::PostgreSQLPlugin::new() }),
        Box::new(MySQLPluginWrapper { inner: taafi_plugin_mysql::MySQLPlugin::new() }),
        Box::new(MongoDBPluginWrapper { inner: taafi_plugin_mongodb::MongoDBPlugin::new() }),
        Box::new(RedisPluginWrapper { inner: taafi_plugin_redis::RedisPlugin::new() }),
        Box::new(KafkaPluginWrapper { inner: taafi_plugin_kafka::KafkaPlugin::new() }),
    ]
}
