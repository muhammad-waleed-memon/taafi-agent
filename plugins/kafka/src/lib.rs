// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use rskafka::client::{ClientBuilder, Client};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct KafkaPlugin {
    client: Arc<Mutex<Option<Client>>>,
}

impl KafkaPlugin {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, connection_string: &str) -> Result<(), String> {
        let brokers: Vec<String> = connection_string.split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let client = ClientBuilder::new(brokers)
            .build()
            .await
            .map_err(|e| format!("Failed to create Kafka client: {}", e))?;

        let mut lock = self.client.lock().await;
        *lock = Some(client);
        info!("Kafka connected successfully.");
        Ok(())
    }

    pub async fn is_healthy(&self) -> bool {
        let lock = self.client.lock().await;
        lock.is_some()
    }
}
