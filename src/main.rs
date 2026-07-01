// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

mod config;
mod logger;
mod models;
mod crypto_engine;
mod memory_engine;
mod metrics_collector;
mod deadlock_detector;
mod fix_executor;
mod sandbox_engine;
mod offline_queue;
mod heartbeat;
mod grpc_server;
mod http_client;
mod cloud_provider;
mod plugin_manager;
mod plugin_loader;
mod agent;

use clap::{Parser, Subcommand};
use config::Config;
use logger::init_logger;
use cloud_provider::AlibabaCloudClient;
use heartbeat::HeartbeatService;
use agent::Agent;
use tracing::{info, warn, error};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "taafi-agent")]
#[command(author = "Muhammad Waleed")]
#[command(version = "0.1.0")]
#[command(about = "TAAFI AI - Self-Learning SRE Agent Daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the agent daemon core loop
    Run {
        /// Optional path to custom configuration file
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Print local agent status details
    Status,
    /// Print software version
    Version,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run { .. } => {
            // Load config
            let config = Config::load()?;
            
            // Set RUST_LOG env before logger initialization
            std::env::set_var("RUST_LOG", &config.rust_log);
            init_logger()?;

            info!("============================================================");
            info!(" TAAFI AI Agent Daemon Starting...");
            info!(" Version: 0.1.0");
            info!(" Agent ID: {}", config.agent_id);
            info!(" Orchestrator URL: {}", config.orchestrator_url);
            info!(" Database URL: {}", config.database_url);
            info!("============================================================");

            // Fetch Alibaba Cloud instance metadata to verify GDPR compliance
            let alibaba_client = AlibabaCloudClient::new();
            match alibaba_client.get_instance_metadata().await {
                Ok(metadata) => {
                    info!("Alibaba Cloud Deployment Proof: Instance={}, Region={}, Zone={}", 
                        metadata.instance_id, metadata.region_id, metadata.zone_id);
                    
                    if let Err(e) = alibaba_client.verify_gdpr_compliance(&metadata) {
                        error!("GDPR COMPLIANCE FAILURE: {:?}", e);
                        // Exit or raise security exception for banking compliance
                        return Err(Box::new(e));
                    }
                }
                Err(e) => {
                    warn!("Alibaba Cloud Metadata check skipped: Not running on Alibaba ECS or Metadata service is unavailable: {:?}", e);
                }
            }

            // Start agent
            let agent = Arc::new(Agent::new(config.clone()).await?);
            
            // Start heartbeat service
            let heartbeat = HeartbeatService::new(
                config.agent_id.clone(),
                config.orchestrator_url.clone(),
                "unknown".to_string(), // Will be updated on connect
            );
            heartbeat.start().await;

            // Start local gRPC server (for debug & manual CLI inspection)
            let status = agent.status();
            let grpc_addr = "127.0.0.1:50052".parse()?;
            tokio::spawn(async move {
                if let Err(e) = grpc_server::start_server(grpc_addr, status).await {
                    error!("Local gRPC Server encountered an error: {:?}", e);
                }
            });

            // Start Agent Main Loop
            agent.run().await?;
        }
        Commands::Status => {
            let config = Config::load()?;
            println!("Agent Status: Querying local status...");
            println!("Agent ID: {}", config.agent_id);
            println!("Orchestrator URL: {}", config.orchestrator_url);
            println!("Database: {}", config.database_url);
        }
        Commands::Version => {
            println!("TAAFI AI Agent Daemon v0.1.0 (Hackathon Track 4 - Autopilot Agent)");
        }
    }

    Ok(())
}
