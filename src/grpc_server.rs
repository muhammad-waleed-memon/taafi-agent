// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{AgentStatus, Incident, Fix, FixResult};
use crate::heartbeat::agent_grpc;
use agent_grpc::agent_service_server::{AgentService, AgentServiceServer};
use agent_grpc::{
    MetricsRequest, MetricsResponse, IncidentReport, IncidentResponse,
    FixRequest, FixResponse, HeartbeatRequest, HeartbeatResponse,
    StatusRequest, StatusResponse, FixData
};
use tonic::{Request, Response, Status};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct AgentServiceImpl {
    status: Arc<RwLock<AgentStatus>>,
}

impl AgentServiceImpl {
    pub fn new(status: Arc<RwLock<AgentStatus>>) -> Self {
        Self { status }
    }
}

#[tonic::async_trait]
impl AgentService for AgentServiceImpl {
    async fn report_metrics(&self, request: Request<MetricsRequest>) -> Result<Response<MetricsResponse>, Status> {
        let req = request.into_inner();
        info!("Received report_metrics from agent: {} containing {} metrics", req.agent_id, req.metrics.len());
        Ok(Response::new(MetricsResponse { success: true }))
    }

    async fn report_incident(&self, request: Request<IncidentReport>) -> Result<Response<IncidentResponse>, Status> {
        let req = request.into_inner();
        info!("Received report_incident from agent: {:?}", req.agent_id);
        
        let mut status_lock = self.status.write().await;
        status_lock.incident_count += 1;

        Ok(Response::new(IncidentResponse {
            success: true,
            incident_id: req.incident.map(|i| i.id).unwrap_or_default(),
            has_suggested_fix: false,
            suggested_fix: None,
        }))
    }

    async fn request_fix(&self, request: Request<FixRequest>) -> Result<Response<FixResponse>, Status> {
        let req = request.into_inner();
        info!("Received request_fix from agent: {:?}", req.agent_id);

        let mut status_lock = self.status.write().await;
        status_lock.fix_count += 1;

        Ok(Response::new(FixResponse {
            success: true,
            message: "Fix request logged".to_string(),
            execution_time_ms: 0,
        }))
    }

    async fn heartbeat(&self, request: Request<HeartbeatRequest>) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        info!("Received heartbeat from: {}", req.agent_id);
        Ok(Response::new(HeartbeatResponse { ack: true, request_sync: false }))
    }

    async fn get_status(&self, _request: Request<StatusRequest>) -> Result<Response<StatusResponse>, Status> {
        let s = self.status.read().await;
        Ok(Response::new(StatusResponse {
            agent_id: s.agent_id.clone(),
            status: s.status.clone(),
            uptime_secs: s.uptime_secs,
            db_type: s.db_type.clone(),
            connected: s.connected,
            incident_count: s.incident_count,
            fix_count: s.fix_count,
        }))
    }
}

pub async fn start_server(addr: std::net::SocketAddr, status: Arc<RwLock<AgentStatus>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let service = AgentServiceImpl::new(status);
    
    info!("Starting tonic gRPC AgentService on {}", addr);
    
    tonic::transport::Server::builder()
        .add_service(AgentServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
