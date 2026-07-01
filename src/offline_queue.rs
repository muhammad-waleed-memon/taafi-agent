// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::Incident;
use sqlx::{SqlitePool, Row};
use tracing::{info, error};

pub struct OfflineQueue {
    pool: SqlitePool,
}

impl OfflineQueue {
    pub async fn new(pool: SqlitePool) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let queue = Self { pool };
        queue.init().await?;
        Ok(queue)
    }

    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS offline_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                incident_json TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn enqueue(&self, incident: &Incident) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let incident_json = serde_json::to_string(incident)?;
        
        sqlx::query("INSERT INTO offline_queue (incident_json) VALUES (?)")
            .bind(incident_json)
            .execute(&self.pool)
            .await?;

        info!("Enqueued incident {} in offline storage", incident.id);
        Ok(())
    }

    pub async fn dequeue_all(&self) -> Result<Vec<(i64, Incident)>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query("SELECT id, incident_json FROM offline_queue ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut incidents = Vec::new();
        for r in rows {
            let id: i64 = r.get("id");
            let json_str: String = r.get("incident_json");
            match serde_json::from_str::<Incident>(&json_str) {
                Ok(inc) => incidents.push((id, inc)),
                Err(e) => {
                    error!("Corrupted incident JSON in offline queue (id: {}): {:?}", id, e);
                    // delete corrupted
                    let _ = sqlx::query("DELETE FROM offline_queue WHERE id = ?").bind(id).execute(&self.pool).await;
                }
            }
        }

        Ok(incidents)
    }

    pub async fn remove(&self, id: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("DELETE FROM offline_queue WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM offline_queue")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }
}
