// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use mongodb::{Client, options::ClientOptions};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use bson::Document;

#[derive(Clone)]
pub struct MongoDBPlugin {
    client: Arc<Mutex<Option<Client>>>,
}

impl MongoDBPlugin {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self, connection_string: &str) -> Result<(), String> {
        let client_options = ClientOptions::parse(connection_string)
            .await
            .map_err(|e| format!("Failed to parse connection string: {}", e))?;
        
        let client = Client::with_options(client_options)
            .map_err(|e| format!("Failed to create client: {}", e))?;

        // Ping the database
        let db = client.database("admin");
        let _ping_res = db.run_command(bson::doc! { "ping": 1 }, None)
            .await
            .map_err(|e| format!("MongoDB connection check failed: {}", e))?;

        let mut lock = self.client.lock().await;
        *lock = Some(client);
        info!("MongoDB connected successfully.");
        Ok(())
    }

    pub async fn is_healthy(&self) -> bool {
        let lock = self.client.lock().await;
        if let Some(ref client) = *lock {
            let db = client.database("admin");
            db.run_command(bson::doc! { "ping": 1 }, None).await.is_ok()
        } else {
            false
        }
    }

    pub async fn run_command(&self, db_name: &str, command: Document) -> Result<Document, String> {
        let lock = self.client.lock().await;
        if let Some(ref client) = *lock {
            let db = client.database(db_name);
            db.run_command(command, None)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("Not connected".to_string())
        }
    }
}
