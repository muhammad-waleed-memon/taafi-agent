// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{Metric, Incident, Fix, DryRunResult, PluginError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait Plugin: Send + Sync + ClonePlugin {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn supported_backends(&self) -> Vec<&'static str>;

    async fn connect(&self, connection_string: &str) -> Result<(), PluginError>;
    async fn disconnect(&self) -> Result<(), PluginError>;
    async fn is_healthy(&self) -> bool;

    async fn collect_metrics(&self) -> Vec<Metric>;
    async fn detect_incidents(&self) -> Vec<Incident>;
    async fn suggest_fixes(&self, incident: &Incident) -> Vec<Fix>;
    async fn apply_fix(&self, fix: &Fix) -> Result<DryRunResult, PluginError>;
    async fn dry_run(&self, fix: &Fix) -> Result<DryRunResult, PluginError>;
}

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, plugin: Box<dyn Plugin>) {
        let mut lock = self.plugins.write().await;
        for backend in plugin.supported_backends() {
            lock.insert(backend.to_string(), plugin.clone_box());
        }
    }

    pub async fn get_plugin(&self, backend: &str) -> Option<Box<dyn Plugin>> {
        let lock = self.plugins.read().await;
        lock.get(backend).map(|p| p.clone_box())
    }

    pub async fn list_plugins(&self) -> Vec<String> {
        let lock = self.plugins.read().await;
        lock.values().map(|p| format!("{}@{}", p.name(), p.version())).collect()
    }

    pub async fn route_incident(&self, incident: &Incident) -> Option<Box<dyn Plugin>> {
        // Simple routing based on incident description or metadata
        // For simplicity, routing by matching backends to plugin
        let db_type = if incident.description.to_lowercase().contains("postgres") {
            "postgresql"
        } else if incident.description.to_lowercase().contains("mysql") {
            "mysql"
        } else if incident.description.to_lowercase().contains("mongo") {
            "mongodb"
        } else if incident.description.to_lowercase().contains("redis") {
            "redis"
        } else if incident.description.to_lowercase().contains("kafka") {
            "kafka"
        } else {
            "universal"
        };

        self.get_plugin(db_type).await
    }
}

// Clone helper trait to allow cloning Box<dyn Plugin>
pub trait ClonePlugin {
    fn clone_box(&self) -> Box<dyn Plugin>;
}

impl<T> ClonePlugin for T
where
    T: 'static + Plugin + Clone,
{
    fn clone_box(&self) -> Box<dyn Plugin> {
        Box::new(self.clone())
    }
}
