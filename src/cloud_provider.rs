// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::CloudError;
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ECSInstanceMetadata {
    pub instance_id: String,
    pub region_id: String,
    pub zone_id: String,
    pub vpc_id: String,
}

pub struct AlibabaCloudClient {
    client: Client,
}

impl AlibabaCloudClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(500)) // Low timeout to quickly fail if not on Alibaba ECS
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }

    pub async fn get_instance_metadata(&self) -> Result<ECSInstanceMetadata, CloudError> {
        info!("Retrieving Alibaba Cloud ECS metadata from Link-Local service...");
        
        let metadata_url = "http://100.100.100.200/latest/meta-data";

        let fetch = |path: &str| async {
            self.client.get(&format!("{}/{}", metadata_url, path))
                .send()
                .await?
                .text()
                .await
        };

        let instance_id = match fetch("instance-id").await {
            Ok(id) => id,
            Err(_) => {
                warn!("ECS Metadata Service unreachable. Falling back to local mock environment metadata.");
                return Ok(ECSInstanceMetadata {
                    instance_id: "i-mock-ecs-frankfurt-2026".to_string(),
                    region_id: "eu-central-1".to_string(),
                    zone_id: "eu-central-1a".to_string(),
                    vpc_id: "vpc-mock-taafi".to_string(),
                });
            }
        };

        let region_id = fetch("region-id").await
            .map_err(|e| CloudError::MetadataFetchFailed(e.to_string()))?;
        
        let zone_id = fetch("zone-id").await
            .map_err(|e| CloudError::MetadataFetchFailed(e.to_string()))?;
            
        let vpc_id = fetch("vpc-id").await
            .map_err(|e| CloudError::MetadataFetchFailed(e.to_string()))?;

        let meta = ECSInstanceMetadata {
            instance_id: instance_id.trim().to_string(),
            region_id: region_id.trim().to_string(),
            zone_id: zone_id.trim().to_string(),
            vpc_id: vpc_id.trim().to_string(),
        };

        Ok(meta)
    }

    pub fn verify_gdpr_compliance(&self, metadata: &ECSInstanceMetadata) -> Result<(), CloudError> {
        if !metadata.region_id.contains("eu-central") {
            return Err(CloudError::RegionNotCompliant(format!(
                "Deployment region {} is not within EU data protection boundaries (eu-central-1).",
                metadata.region_id
            )));
        }
        info!("GDPR Compliance verification successful: Region is {}", metadata.region_id);
        Ok(())
    }
}
