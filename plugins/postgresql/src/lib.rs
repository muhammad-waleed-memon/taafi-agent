// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use tokio_postgres::{Client, NoTls};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error};

#[derive(Clone)]
pub struct PostgreSQLPlugin {
    client: Arc<Mutex<Option<Client>>>,
}

impl PostgreSQLPlugin {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, connection_string: &str) -> Result<(), String> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        // Spawn connection handler in background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });

        let mut lock = self.client.lock().await;
        *lock = Some(client);
        info!("PostgreSQL connected successfully.");
        Ok(())
    }

    pub async fn is_healthy(&self) -> bool {
        let lock = self.client.lock().await;
        if let Some(ref client) = *lock {
            client.simple_query("SELECT 1").await.is_ok()
        } else {
            false
        }
    }

    pub async fn execute_query(&self, sql: &str) -> Result<u64, String> {
        let lock = self.client.lock().await;
        if let Some(ref client) = *lock {
            client.execute(sql, &[])
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }

    pub async fn query_rows(&self, sql: &str) -> Result<Vec<tokio_postgres::Row>, String> {
        let lock = self.client.lock().await;
        if let Some(ref client) = *lock {
            client.query(sql, &[])
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }
}
