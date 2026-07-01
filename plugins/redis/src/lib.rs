// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use redis::{Client, aio::MultiplexedConnection, AsyncCommands};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct RedisPlugin {
    client: Arc<Mutex<Option<Client>>>,
    conn: Arc<Mutex<Option<MultiplexedConnection>>>,
}

impl RedisPlugin {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            conn: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, connection_string: &str) -> Result<(), String> {
        let client = Client::open(connection_string)
            .map_err(|e| format!("Failed to parse Redis URL: {}", e))?;
        
        let conn = client.get_multiplexed_tokio_connection()
            .await
            .map_err(|e| format!("Failed to connect to Redis: {}", e))?;

        let mut lock_client = self.client.lock().await;
        *lock_client = Some(client);

        let mut lock_conn = self.conn.lock().await;
        *lock_conn = Some(conn);

        info!("Redis/Valkey connected successfully.");
        Ok(())
    }

    pub async fn is_healthy(&self) -> bool {
        let mut lock = self.conn.lock().await;
        if let Some(ref mut conn) = *lock {
            let res: Result<String, _> = redis::cmd("PING").query_async(conn).await;
            res.is_ok()
        } else {
            false
        }
    }

    pub async fn execute_cmd<T: redis::FromRedisValue>(&self, cmd_name: &str, args: &[&str]) -> Result<T, String> {
        let mut lock = self.conn.lock().await;
        if let Some(ref mut conn) = *lock {
            let mut cmd = redis::cmd(cmd_name);
            for arg in args {
                cmd.arg(*arg);
            }
            cmd.query_async(conn).await.map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }
}
