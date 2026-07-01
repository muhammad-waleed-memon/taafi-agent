// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use crate::models::{Incident, Fix, MemoryMatch, FixType};
use sha3::{Sha3_256, Digest};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use tracing::{info, warn};

pub struct MemoryEngine {
    pool: SqlitePool,
}

impl MemoryEngine {
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pool = SqlitePool::connect(&format!("sqlite:{}", db_path)).await;
        let pool = match pool {
            Ok(p) => p,
            Err(_) => {
                // Try creating file if it doesn't exist
                let options = sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(db_path)
                    .create_if_missing(true);
                SqlitePool::connect_with(options).await?
            }
        };

        let engine = Self { pool };
        engine.init().await?;
        Ok(engine)
    }

    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS incident_patterns (
                pattern_hash TEXT PRIMARY KEY,
                incident_type TEXT NOT NULL,
                affected_tables TEXT,
                fix_type TEXT,
                fix_sql TEXT,
                success_count INTEGER DEFAULT 0,
                failure_count INTEGER DEFAULT 0,
                last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub fn normalize_query(&self, query: &str) -> String {
        let re_strings = regex::Regex::new(r"'(?:[^'\\]|\\.)*'").unwrap();
        let query = re_strings.replace_all(query, "?");

        let re_nums = regex::Regex::new(r"\b\d+\b").unwrap();
        let query = re_nums.replace_all(&query, "?");

        let re_whitespace = regex::Regex::new(r"\s+").unwrap();
        let query = re_whitespace.replace_all(&query, " ");

        query.trim().to_lowercase()
    }

    pub fn compute_pattern_hash(&self, incident_type: &str, tables: &[String], query: &str) -> String {
        let mut sorted_tables = tables.to_vec();
        sorted_tables.sort();
        let tables_str = sorted_tables.join(",");
        let normalized = self.normalize_query(query);

        let input = format!("{}:{}:{}", incident_type, tables_str, normalized);
        let mut hasher = Sha3_256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let hex = result.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        hex[..16].to_string()
    }

    pub async fn recall(&self, incident: &Incident) -> Result<Option<MemoryMatch>, Box<dyn std::error::Error + Send + Sync>> {
        let hash = self.compute_pattern_hash(
            &incident.incident_type.to_string(),
            &incident.affected_tables,
            incident.query.as_deref().unwrap_or("")
        );

        let row = sqlx::query("SELECT pattern_hash, fix_type, fix_sql, success_count, failure_count FROM incident_patterns WHERE pattern_hash = ?")
            .bind(&hash)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            let success_count: i64 = r.get("success_count");
            let failure_count: i64 = r.get("failure_count");
            let total = success_count + failure_count;
            let success_rate = if total > 0 {
                success_count as f64 / total as f64
            } else {
                0.0
            };

            let fix_type_str: String = r.get("fix_type");
            let fix_type = match fix_type_str.as_str() {
                "CreateIndex" => FixType::CreateIndex,
                "Analyze" => FixType::Analyze,
                "Vacuum" => FixType::Vacuum,
                "Reindex" => FixType::Reindex,
                "TerminateBackend" => FixType::TerminateBackend,
                "ConfigChange" => FixType::ConfigChange,
                "KillOp" => FixType::KillOp,
                _ => FixType::Custom,
            };

            if success_rate > 0.95 && total >= 3 {
                info!("Memory recall exact match for pattern hash {}: success_rate = {}", hash, success_rate);
                return Ok(Some(MemoryMatch {
                    pattern_hash: hash,
                    fix_type,
                    fix_sql: r.get("fix_sql"),
                    confidence: 0.98,
                    success_rate,
                }));
            }
        }

        // Try similarity search
        self.similarity_search(incident).await
    }

    pub async fn remember(&self, incident: &Incident, fix: &Fix, success: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let hash = self.compute_pattern_hash(
            &incident.incident_type.to_string(),
            &incident.affected_tables,
            incident.query.as_deref().unwrap_or("")
        );

        let fix_type_str = fix.fix_type.to_string();
        let fix_sql = fix.sql.clone();

        let inc_success = if success { 1 } else { 0 };
        let inc_failure = if success { 0 } else { 1 };

        sqlx::query(
            "INSERT INTO incident_patterns (pattern_hash, incident_type, affected_tables, fix_type, fix_sql, success_count, failure_count, last_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(pattern_hash) DO UPDATE SET
                success_count = success_count + ?,
                failure_count = failure_count + ?,
                last_seen = CURRENT_TIMESTAMP"
        )
        .bind(&hash)
        .bind(incident.incident_type.to_string())
        .bind(incident.affected_tables.join(","))
        .bind(fix_type_str)
        .bind(fix_sql)
        .bind(inc_success)
        .bind(inc_failure)
        .bind(inc_success)
        .bind(inc_failure)
        .execute(&self.pool)
        .await?;

        info!("Pattern {} remembered (success = {})", hash, success);
        Ok(())
    }

    pub async fn similarity_search(&self, incident: &Incident) -> Result<Option<MemoryMatch>, Box<dyn std::error::Error + Send + Sync>> {
        let target_query = self.normalize_query(incident.query.as_deref().unwrap_or(""));
        let target_words = tokenize(&target_query);

        let rows = sqlx::query("SELECT pattern_hash, incident_type, affected_tables, fix_type, fix_sql, success_count, failure_count FROM incident_patterns")
            .fetch_all(&self.pool)
            .await?;

        let mut best_match: Option<MemoryMatch> = None;
        let mut highest_sim = 0.0;

        for r in rows {
            let row_incident_type: String = r.get("incident_type");
            if row_incident_type != incident.incident_type.to_string() {
                continue;
            }

            let fix_sql_opt: Option<String> = r.get("fix_sql");
            let fix_sql = fix_sql_opt.clone().unwrap_or_default();
            let row_words = tokenize(&self.normalize_query(&fix_sql));

            let sim = cosine_similarity(&target_words, &row_words);
            if sim > 0.90 && sim > highest_sim {
                highest_sim = sim;
                let success_count: i64 = r.get("success_count");
                let failure_count: i64 = r.get("failure_count");
                let total = success_count + failure_count;
                let success_rate = if total > 0 {
                    success_count as f64 / total as f64
                } else {
                    0.0
                };

                let fix_type_str: String = r.get("fix_type");
                let fix_type = match fix_type_str.as_str() {
                    "CreateIndex" => FixType::CreateIndex,
                    "Analyze" => FixType::Analyze,
                    "Vacuum" => FixType::Vacuum,
                    "Reindex" => FixType::Reindex,
                    "TerminateBackend" => FixType::TerminateBackend,
                    "ConfigChange" => FixType::ConfigChange,
                    "KillOp" => FixType::KillOp,
                    _ => FixType::Custom,
                };

                best_match = Some(MemoryMatch {
                    pattern_hash: r.get("pattern_hash"),
                    fix_type,
                    fix_sql: fix_sql_opt,
                    confidence: sim,
                    success_rate,
                });
            }
        }

        if let Some(ref m) = best_match {
            info!("Memory recall similarity match found: pattern_hash = {}, similarity = {}", m.pattern_hash, highest_sim);
        }

        Ok(best_match)
    }
}

fn tokenize(text: &str) -> HashMap<String, f64> {
    let mut tokens = HashMap::new();
    for word in text.split_whitespace() {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        if !word.is_empty() {
            *tokens.entry(word).or_insert(0.0) += 1.0;
        }
    }
    tokens
}

fn cosine_similarity(v1: &HashMap<String, f64>, v2: &HashMap<String, f64>) -> f64 {
    let mut dot_product = 0.0;
    for (k, val1) in v1 {
        if let Some(val2) = v2.get(k) {
            dot_product += val1 * val2;
        }
    }

    let norm1 = v1.values().map(|x| x * x).sum::<f64>().sqrt();
    let norm2 = v2.values().map(|x| x * x).sum::<f64>().sqrt();

    if norm1 > 0.0 && norm2 > 0.0 {
        dot_product / (norm1 * norm2)
    } else {
        0.0
    }
}
