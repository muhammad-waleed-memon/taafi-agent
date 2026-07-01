// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use mysql_async::{Pool, Opts, prelude::*};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct MySQLPlugin {
    pool: Arc<Mutex<Option<Pool>>>,
}

impl MySQLPlugin {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, connection_string: &str) -> Result<(), String> {
        let opts = Opts::from_url(connection_string)
            .map_err(|e| format!("Invalid connection URL: {}", e))?;
        
        let pool = Pool::new(opts);
        
        // Check health
        let mut conn = pool.get_conn().await
            .map_err(|e| format!("Failed to get connection: {}", e))?;
        
        let _res: Option<u32> = conn.query_first("SELECT 1")
            .await
            .map_err(|e| format!("MySQL connection check failed: {}", e))?;

        let mut lock = self.pool.lock().await;
        *lock = Some(pool);
        info!("MySQL connected successfully.");
        Ok(())
    }

    pub async fn is_healthy(&self) -> bool {
        let lock = self.pool.lock().await;
        if let Some(ref pool) = *lock {
            if let Ok(mut conn) = pool.get_conn().await {
                let res: Result<Option<u32>, _> = conn.query_first("SELECT 1").await;
                return res.is_ok();
            }
        }
        false
    }

    pub async fn execute_query(&self, sql: &str) -> Result<u64, String> {
        let lock = self.pool.lock().await;
        if let Some(ref pool) = *lock {
            let mut conn = pool.get_conn().await.map_err(|e| e.to_string())?;
            conn.query_drop(sql).await.map_err(|e| e.to_string())?;
            // mysql_async query_drop doesn't return affected rows, but we can query affected rows or return 0
            Ok(0)
        } else {
            Err("Not connected".to_string())
        }
    }
}
