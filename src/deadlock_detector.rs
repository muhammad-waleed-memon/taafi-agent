// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{Incident, IncidentType, Severity};
use crate::plugin_manager::Plugin;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashSet;

pub struct DeadlockDetector;

impl DeadlockDetector {
    pub fn new() -> Self {
        Self
    }

    pub async fn detect(&self, plugin: &dyn Plugin) -> Vec<Incident> {
        let incidents = plugin.detect_incidents().await;
        let mut deadlocks = Vec::new();

        for inc in incidents {
            if inc.incident_type == IncidentType::Deadlock {
                deadlocks.push(inc);
            }
        }

        // Wait-graph cycle check (simplification)
        if deadlocks.len() > 1 {
            if let Some(cycle_incident) = self.analyze_wait_graph(&deadlocks) {
                return vec![cycle_incident];
            }
        }

        deadlocks
    }

    fn analyze_wait_graph(&self, incidents: &[Incident]) -> Option<Incident> {
        // Look for cycles in waits (e.g. Transaction A waits for B, B waits for A)
        // Using incident description to find deadlock/cycle patterns
        let mut processes = HashSet::new();
        for inc in incidents {
            if inc.description.contains("wait") || inc.description.contains("lock") {
                processes.insert(&inc.agent_id);
            }
        }

        if !processes.is_empty() {
            return Some(Incident {
                id: Uuid::new_v4(),
                incident_type: IncidentType::Deadlock,
                severity: Severity::Critical,
                description: format!("Wait graph deadlock detected involving {} agents.", processes.len()),
                affected_tables: incidents.iter().flat_map(|i| i.affected_tables.clone()).collect(),
                query: incidents.first().and_then(|i| i.query.clone()),
                timestamp: Utc::now(),
                agent_id: incidents.first().map(|i| i.agent_id.clone()).unwrap_or_default(),
                resolved: false,
            });
        }

        None
    }
}
