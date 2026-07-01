// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedDb {
    PostgreSQL,
    MySQL,
    MongoDB,
    Redis,
    Kafka,
    Unknown,
}

#[derive(Clone)]
pub struct UniversalPlugin {
    pub connection_string: String,
    pub db_type: DetectedDb,
}

impl UniversalPlugin {
    pub fn new() -> Self {
        Self {
            connection_string: String::new(),
            db_type: DetectedDb::Unknown,
        }
    }

    pub fn detect_db_type(&self, conn_str: &str) -> DetectedDb {
        // Parse port from connection string
        // Format: host:port, or postgres://user:pass@host:port/db, etc.
        let port = if conn_str.contains("5432") {
            5432
        } else if conn_str.contains("3306") {
            3306
        } else if conn_str.contains("27017") {
            27017
        } else if conn_str.contains("6379") {
            6379
        } else if conn_str.contains("9092") {
            9092
        } else {
            0
        };

        match port {
            5432 => DetectedDb::PostgreSQL,
            3306 => DetectedDb::MySQL,
            27017 => DetectedDb::MongoDB,
            6379 => DetectedDb::Redis,
            9092 => DetectedDb::Kafka,
            _ => {
                // Try protocol handshake or fallback to TCP port scan
                DetectedDb::Unknown
            }
        }
    }

    pub async fn connect(&mut self, connection_string: &str) -> Result<(), String> {
        self.connection_string = connection_string.to_string();
        self.db_type = self.detect_db_type(connection_string);
        
        // Try TCP connection check
        let addr = if connection_string.contains("://") {
            // strip scheme
            let parts: Vec<&str> = connection_string.split("://").collect();
            parts.get(1).cloned().unwrap_or(connection_string)
        } else {
            connection_string
        };

        // Extract host and port
        let host_port = if addr.contains("@") {
            let parts: Vec<&str> = addr.split('@').collect();
            parts.get(1).cloned().unwrap_or(addr)
        } else {
            addr
        };

        let host_port = host_port.split('/').next().unwrap_or(host_port);

        if let Ok(mut addrs) = host_port.to_socket_addrs() {
            if let Some(socket_addr) = addrs.next() {
                if TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)).is_ok() {
                    return Ok(());
                }
            }
        }

        // Return error if TCP connect fails
        Err(format!("Failed to connect to {}", connection_string))
    }

    pub async fn is_healthy(&self) -> bool {
        !self.connection_string.is_empty()
    }
}
