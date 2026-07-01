// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

use reqwest::{Client, Response, RequestBuilder};
use std::time::Duration;
use tracing::{warn, info};

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    pub async fn execute_with_retry(&self, req_builder: RequestBuilder) -> Result<Response, reqwest::Error> {
        let mut retries = 0;
        let max_retries = 3;
        let mut delay = Duration::from_secs(1);

        loop {
            // RequestBuilder cannot be cloned directly easily, so we usually clone requests or pass closures
            // For simplicity in a generic request, we can clone request builders or build the request beforehand.
            // Let's copy request fields if we need to rebuild, or construct it.
            // Under normal usage, reqwest allows request.try_clone().
            let req = req_builder.try_clone();
            
            let res = match req {
                Some(r) => r.send().await,
                None => {
                    // Fallback to sending without clone if not cloneable (first try only)
                    return req_builder.send().await;
                }
            };

            match res {
                Ok(response) => {
                    if response.status().is_server_error() && retries < max_retries {
                        warn!("HTTP request server error status: {}. Retrying in {:?}...", response.status(), delay);
                        tokio::time::sleep(delay).await;
                        retries += 1;
                        delay *= 2;
                        continue;
                    }
                    return Ok(response);
                }
                Err(err) => {
                    if retries < max_retries && (err.is_timeout() || err.is_connect()) {
                        warn!("HTTP request network error: {:?}. Retrying in {:?}...", err, delay);
                        tokio::time::sleep(delay).await;
                        retries += 1;
                        delay *= 2;
                        continue;
                    }
                    return Err(err);
                }
            }
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
