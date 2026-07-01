// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{AgentStatus, Incident, Fix, FixResult, PluginError};
use crate::config::Config;
use crate::plugin_manager::{PluginManager, Plugin};
use crate::plugin_loader::load_plugins;
use crate::memory_engine::MemoryEngine;
use crate::crypto_engine::PqCryptoEngine;
use crate::metrics_collector::MetricsCollector;
use crate::deadlock_detector::DeadlockDetector;
use crate::fix_executor::FixExecutor;
use crate::sandbox_engine::SandboxEngine;
use crate::offline_queue::OfflineQueue;
use crate::heartbeat::agent_grpc::{self, MetricsRequest, MetricData, IncidentReport, IncidentData};
use crate::heartbeat::agent_grpc::agent_service_client::AgentServiceClient;

use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::Utc;

pub struct Agent {
    config: Config,
    plugin_manager: Arc<PluginManager>,
    memory_engine: Arc<MemoryEngine>,
    crypto_engine: Arc<PqCryptoEngine>,
    status: Arc<RwLock<AgentStatus>>,
    offline_queue: Arc<OfflineQueue>,
    start_time: std::time::Instant,
}

impl Agent {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let plugin_manager = Arc::new(PluginManager::new());
        
        // Register plugins
        let plugins = load_plugins();
        for p in plugins {
            plugin_manager.register(p).await;
        }

        // Initialize SQLite memory engine
        let memory_engine = Arc::new(MemoryEngine::new(&config.memory_db_path).await?);
        
        // Initialize offline queue SQLite pool
        let sqlite_pool = sqlx::SqlitePool::connect(&format!("sqlite:{}", config.memory_db_path)).await?;
        let offline_queue = Arc::new(OfflineQueue::new(sqlite_pool).await?);

        // Crypto engine
        let crypto_engine = Arc::new(PqCryptoEngine::new()?);

        // Status
        let status = Arc::new(RwLock::new(AgentStatus {
            agent_id: config.agent_id.clone(),
            status: "initializing".to_string(),
            uptime_secs: 0,
            db_type: "unknown".to_string(),
            connected: false,
            incident_count: 0,
            fix_count: 0,
        }));

        Ok(Self {
            config,
            plugin_manager,
            memory_engine,
            crypto_engine,
            status,
            offline_queue,
            start_time: std::time::Instant::now(),
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting TAAFI Agent core loop...");
        
        // Try connecting to database
        let db_type = if self.config.database_url.contains("5432") {
            "postgresql"
        } else if self.config.database_url.contains("3306") {
            "mysql"
        } else if self.config.database_url.contains("27017") {
            "mongodb"
        } else if self.config.database_url.contains("6379") {
            "redis"
        } else if self.config.database_url.contains("9092") {
            "kafka"
        } else {
            "universal"
        };

        {
            let mut s = self.status.write().await;
            s.status = "connecting".to_string();
            s.db_type = db_type.to_string();
        }

        if let Some(plugin) = self.plugin_manager.get_plugin(db_type).await {
            info!("Connecting plugin {}...", plugin.name());
            if let Err(e) = plugin.connect(&self.config.database_url).await {
                error!("Plugin failed to connect to database: {:?}", e);
            } else {
                info!("Plugin {} connected successfully.", plugin.name());
                let mut s = self.status.write().await;
                s.connected = true;
                s.status = "running".to_string();
            }
        } else {
            warn!("No specific plugin found to handle database URL. Using default/universal plugin.");
            if let Some(plugin) = self.plugin_manager.get_plugin("universal").await {
                if let Err(e) = plugin.connect(&self.config.database_url).await {
                    error!("Universal plugin failed to connect: {:?}", e);
                } else {
                    info!("Universal plugin connected successfully.");
                    let mut s = self.status.write().await;
                    s.connected = true;
                    s.status = "running".to_string();
                }
            }
        }

        let collector = MetricsCollector::new();
        let deadlock_detector = DeadlockDetector::new();
        let fix_executor = FixExecutor::new();
        let sandbox_engine = SandboxEngine::new();

        loop {
            // Update uptime
            {
                let mut s = self.status.write().await;
                s.uptime_secs = self.start_time.elapsed().as_secs();
            }

            // 1. Collect Metrics
            let system_metrics = collector.collect_system_metrics();
            info!("Collected {} system metrics.", system_metrics.len());

            // 2. Detect Incidents
            if let Some(plugin) = self.plugin_manager.get_plugin(db_type).await {
                let incidents = deadlock_detector.detect(plugin.as_ref()).await;
                if !incidents.is_empty() {
                    info!("Detected {} database incident(s).", incidents.len());
                    for incident in incidents {
                        self.process_incident(incident, plugin.as_ref(), &fix_executor, &sandbox_engine).await;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(self.config.metrics_interval_secs)).await;
        }
    }

    async fn process_incident(
        &self,
        incident: Incident,
        plugin: &dyn Plugin,
        fix_executor: &FixExecutor,
        sandbox_engine: &SandboxEngine
    ) {
        // 1. Check memory engine
        match self.memory_engine.recall(&incident).await {
            Ok(Some(mem_match)) => {
                info!("Memory engine matched incident pattern hash {}. Auto-fixing...", mem_match.pattern_hash);
                
                // Formulate Fix
                let fix = Fix {
                    id: Uuid::new_v4(),
                    incident_id: incident.id,
                    fix_type: mem_match.fix_type,
                    description: "Auto-remediation resolved from pattern memory".to_string(),
                    sql: mem_match.fix_sql,
                    command: None,
                    risk_score: 0.1,
                    confidence: mem_match.confidence,
                    rollback_plan: None,
                    approved: true,
                    applied: false,
                };

                // Validate and dry run
                if let Ok(_dry_run) = sandbox_engine.dry_run(&fix, plugin).await {
                    match fix_executor.apply_fix(&fix, plugin).await {
                        Ok(res) => {
                            let _ = self.memory_engine.remember(&incident, &fix, res.success).await;
                            info!("Incident auto-fixed successfully.");
                        }
                        Err(e) => {
                            error!("Auto-fix execution failed: {:?}", e);
                            let _ = self.memory_engine.remember(&incident, &fix, false).await;
                        }
                    }
                }
            }
            _ => {
                info!("No confident memory match found. Escolating incident {} to orchestrator...", incident.id);
                // Queue offline if orchestrator is down
                if let Err(e) = self.offline_queue.enqueue(&incident).await {
                    error!("Failed to enqueue incident in offline queue: {:?}", e);
                }
                
                // Attempt to send to orchestrator
                let channel = tonic::transport::Endpoint::from_shared(self.config.orchestrator_url.clone());
                if let Ok(chan) = channel {
                    if let Ok(conn) = chan.connect().await {
                        let mut client = AgentServiceClient::new(conn);
                        
                        let req = tonic::Request::new(IncidentReport {
                            agent_id: self.config.agent_id.clone(),
                            incident: Some(IncidentData {
                                id: incident.id.to_string(),
                                incident_type: incident.incident_type.to_string(),
                                severity: incident.severity.to_string(),
                                description: incident.description.clone(),
                                affected_tables: incident.affected_tables.clone(),
                                query: incident.query.clone().unwrap_or_default(),
                                timestamp: incident.timestamp.timestamp_millis(),
                                agent_id: self.config.agent_id.clone(),
                                resolved: false,
                            }),
                        });

                        match client.report_incident(req).await {
                            Ok(res) => {
                                let body = res.into_inner();
                                if body.has_suggested_fix {
                                    if let Some(fix_data) = body.suggested_fix {
                                        info!("Orchestrator returned fix proposal: {}", fix_data.description);
                                        // Execute
                                    }
                                }
                            }
                            Err(err) => {
                                error!("Failed to send incident to orchestrator: {:?}", err);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn status(&self) -> Arc<RwLock<AgentStatus>> {
        self.status.clone()
    }
}
