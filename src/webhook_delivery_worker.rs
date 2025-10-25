use crate::types::{DeliveryStatus, HttpMethod, WebhookConfig};
use chrono::Utc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DeliveryTask {
    pub webhook: WebhookConfig,
    pub payload: serde_json::Value,
    pub delivery_id: Uuid,
}

pub struct WebhookDeliveryQueue {
    tx: mpsc::Sender<DeliveryTask>,
}

impl WebhookDeliveryQueue {
    pub fn new(buffer_size: usize) -> (Self, mpsc::Receiver<DeliveryTask>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        (Self { tx }, rx)
    }

    pub async fn enqueue(&self, task: DeliveryTask) -> Result<(), String> {
        self.tx
            .send(task)
            .await
            .map_err(|e| format!("Failed to enqueue webhook delivery: {e}"))
    }
}

/// Background worker that processes webhook deliveries
pub async fn webhook_delivery_worker(
    mut rx: mpsc::Receiver<DeliveryTask>,
    storage_tx: mpsc::Sender<DeliveryStatusUpdate>,
) {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    while let Some(task) = rx.recv().await {
        let result = deliver_webhook_with_retry(
            &http_client,
            &task.webhook,
            &task.payload,
            task.delivery_id,
            &storage_tx,
        )
        .await;

        if let Err(e) = result {
            eprintln!("Webhook delivery failed for {}: {}", task.delivery_id, e);
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeliveryStatusUpdate {
    pub delivery_id: Uuid,
    pub status: DeliveryStatus,
    pub attempts: u32,
    pub response_code: Option<u16>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub delivered_at: Option<chrono::DateTime<Utc>>,
    pub next_retry_at: Option<chrono::DateTime<Utc>>,
}

async fn deliver_webhook_with_retry(
    http_client: &reqwest::Client,
    webhook: &WebhookConfig,
    payload: &serde_json::Value,
    delivery_id: Uuid,
    storage_tx: &mpsc::Sender<DeliveryStatusUpdate>,
) -> Result<(), String> {
    let max_retries = webhook.retry_config.max_retries;
    let mut attempt = 0;

    loop {
        attempt += 1;

        // Update delivery status to in progress
        let _ = storage_tx
            .send(DeliveryStatusUpdate {
                delivery_id,
                status: DeliveryStatus::InProgress,
                attempts: attempt,
                response_code: None,
                response_body: None,
                error_message: None,
                delivered_at: None,
                next_retry_at: None,
            })
            .await;

        // Build HTTP request
        let mut request = match webhook.method {
            HttpMethod::Post => http_client.post(&webhook.url),
            HttpMethod::Put => http_client.put(&webhook.url),
            HttpMethod::Patch => http_client.patch(&webhook.url),
        };

        // Add headers
        for (key, value) in &webhook.headers {
            request = request.header(key, value);
        }

        // Add authentication
        request = match &webhook.auth_type {
            crate::types::WebhookAuthType::None => request,
            crate::types::WebhookAuthType::BearerToken => {
                if let Some(token) = &webhook.auth_credentials {
                    request.bearer_auth(token)
                } else {
                    request
                }
            }
            crate::types::WebhookAuthType::ApiKey => {
                if let Some(api_key) = &webhook.auth_credentials {
                    request.header("X-API-Key", api_key)
                } else {
                    request
                }
            }
            crate::types::WebhookAuthType::BasicAuth => {
                if let Some(creds) = &webhook.auth_credentials {
                    let parts: Vec<&str> = creds.split(':').collect();
                    if parts.len() == 2 {
                        request.basic_auth(parts[0], Some(parts[1]))
                    } else {
                        request
                    }
                } else {
                    request
                }
            }
            crate::types::WebhookAuthType::CustomHeader => {
                // Custom header already added in headers map
                request
            }
        };

        // Set content type and body
        request = request
            .header("Content-Type", "application/json")
            .json(payload);

        // Send request
        match request.send().await {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let response_body = response.text().await.unwrap_or_default();

                if (200..300).contains(&status_code) {
                    // Success
                    let _ = storage_tx
                        .send(DeliveryStatusUpdate {
                            delivery_id,
                            status: DeliveryStatus::Delivered,
                            attempts: attempt,
                            response_code: Some(status_code),
                            response_body: Some(response_body),
                            error_message: None,
                            delivered_at: Some(Utc::now()),
                            next_retry_at: None,
                        })
                        .await;

                    return Ok(());
                } else {
                    // HTTP error
                    if attempt > max_retries {
                        // Max retries reached
                        let _ = storage_tx
                            .send(DeliveryStatusUpdate {
                                delivery_id,
                                status: DeliveryStatus::Failed,
                                attempts: attempt,
                                response_code: Some(status_code),
                                response_body: Some(response_body.clone()),
                                error_message: Some(format!(
                                    "HTTP error {status_code}: {response_body}"
                                )),
                                delivered_at: None,
                                next_retry_at: None,
                            })
                            .await;

                        return Err(format!("HTTP error {status_code} after {attempt} attempts"));
                    }
                }
            }
            Err(e) => {
                // Network error
                if attempt > max_retries {
                    // Max retries reached
                    let _ = storage_tx
                        .send(DeliveryStatusUpdate {
                            delivery_id,
                            status: DeliveryStatus::Failed,
                            attempts: attempt,
                            response_code: None,
                            response_body: None,
                            error_message: Some(format!("Network error: {e}")),
                            delivered_at: None,
                            next_retry_at: None,
                        })
                        .await;

                    return Err(format!("Network error after {attempt} attempts: {e}"));
                }
            }
        }

        // Calculate retry delay with exponential backoff
        let delay_ms = webhook.retry_config.initial_delay_ms as f64
            * webhook
                .retry_config
                .backoff_multiplier
                .powi((attempt - 1) as i32);
        let delay_ms = delay_ms.min(webhook.retry_config.max_delay_ms as f64) as u64;

        // Update status to retrying
        let next_retry = Utc::now() + chrono::Duration::milliseconds(delay_ms as i64);
        let _ = storage_tx
            .send(DeliveryStatusUpdate {
                delivery_id,
                status: DeliveryStatus::Retrying,
                attempts: attempt,
                response_code: None,
                response_body: None,
                error_message: None,
                delivered_at: None,
                next_retry_at: Some(next_retry),
            })
            .await;

        // Wait before retrying
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

/// Storage update worker that processes delivery status updates
pub async fn storage_update_worker<S: crate::storage::StorageBackend + 'static>(
    mut rx: mpsc::Receiver<DeliveryStatusUpdate>,
    storage: std::sync::Arc<std::sync::Mutex<S>>,
) {
    while let Some(update) = rx.recv().await {
        // Update delivery in storage
        if let Ok(storage_guard) = storage.lock() {
            if let Ok(Some(mut delivery)) = storage_guard.get_webhook_delivery(&update.delivery_id)
            {
                delivery.status = update.status;
                delivery.attempts = update.attempts;
                delivery.response_code = update.response_code;
                delivery.response_body = update.response_body;
                delivery.error_message = update.error_message;
                delivery.delivered_at = update.delivered_at;
                delivery.next_retry_at = update.next_retry_at;

                let _ = storage_guard.store_webhook_delivery(&delivery);
            }
        }
    }
}
