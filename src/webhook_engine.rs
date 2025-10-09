use crate::storage::StorageBackend;
use crate::types::{
    WebhookConfig, WebhookDelivery, WebhookPayload, DeliveryStatus, PostActionTrigger,
};
use crate::logging::LoggingEngine;
use crate::webhook_delivery_worker::{WebhookDeliveryQueue, DeliveryTask};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub enum WebhookError {
    StorageError(String),
    DeliveryError(String),
    ConfigurationError(String),
    NetworkError(String),
    AuthenticationError(String),
    ValidationError(String),
}

impl std::fmt::Display for WebhookError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WebhookError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            WebhookError::DeliveryError(msg) => write!(f, "Delivery error: {}", msg),
            WebhookError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            WebhookError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            WebhookError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            WebhookError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for WebhookError {}

pub struct WebhookEngine<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
    logger: LoggingEngine,
    delivery_queue: Option<Arc<WebhookDeliveryQueue>>,
}

impl<S: StorageBackend> WebhookEngine<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        Self {
            storage,
            logger: LoggingEngine::new(),
            delivery_queue: None,
        }
    }

    pub fn with_delivery_queue(mut self, queue: Arc<WebhookDeliveryQueue>) -> Self {
        self.delivery_queue = Some(queue);
        self
    }

    /// Trigger webhooks for a given event
    pub async fn trigger_webhooks(
        &mut self,
        circuit_id: &Uuid,
        trigger_event: PostActionTrigger,
        payload: WebhookPayload,
    ) -> Result<Vec<Uuid>, WebhookError> {
        self.logger.info(
            "webhook_engine",
            "webhook_trigger",
            &format!("Triggering webhooks for circuit {} event {:?}", circuit_id, trigger_event)
        )
        .with_context("circuit_id", circuit_id.to_string())
        .with_context("trigger_event", trigger_event.as_str());

        // Get circuit post-action settings
        let webhooks = {
            let storage = self.storage.lock()
            .map_err(|_| WebhookError::StorageError("Storage mutex poisoned".to_string()))?;
            let circuit = storage.get_circuit(circuit_id)
                .map_err(|e| WebhookError::StorageError(e.to_string()))?
                .ok_or_else(|| WebhookError::ConfigurationError("Circuit not found".to_string()))?;

            // Check if post-action settings are enabled
            let post_settings = match circuit.post_action_settings {
                Some(settings) if settings.enabled => settings,
                _ => return Ok(vec![]), // No webhooks configured or disabled
            };

            // Check if this trigger event is configured
            if !post_settings.trigger_events.contains(&trigger_event) {
                self.logger.info(
                    "webhook_engine",
                    "webhook_skip",
                    &format!("Trigger event {:?} not configured for circuit", trigger_event)
                );
                return Ok(vec![]);
            }

            // Get enabled webhooks
            post_settings.webhooks.into_iter()
                .filter(|w| w.enabled)
                .collect::<Vec<_>>()
        };

        if webhooks.is_empty() {
            return Ok(vec![]);
        }

        let mut delivery_ids = Vec::new();

        // Create deliveries for each webhook
        for webhook in webhooks {
            let delivery_id = self.create_delivery(
                &webhook,
                *circuit_id,
                trigger_event,
                payload.clone(),
            ).await?;

            delivery_ids.push(delivery_id);
        }

        self.logger.info(
            "webhook_engine",
            "webhooks_triggered",
            &format!("Created {} webhook deliveries", delivery_ids.len())
        )
        .with_context("count", delivery_ids.len().to_string());

        Ok(delivery_ids)
    }

    /// Create a webhook delivery and initiate sending
    async fn create_delivery(
        &mut self,
        webhook: &WebhookConfig,
        circuit_id: Uuid,
        trigger_event: PostActionTrigger,
        payload: WebhookPayload,
    ) -> Result<Uuid, WebhookError> {
        // Serialize payload
        let payload_value = serde_json::to_value(&payload)
            .map_err(|e| WebhookError::DeliveryError(format!("Failed to serialize payload: {}", e)))?;

        // Create delivery record
        let mut delivery = WebhookDelivery::new(
            webhook.id,
            circuit_id,
            trigger_event,
            payload_value.clone(),
        );

        // Store delivery
        {
            let mut storage = self.storage.lock()
            .map_err(|_| WebhookError::StorageError("Storage mutex poisoned".to_string()))?;
            storage.store_webhook_delivery(&delivery)
                .map_err(|e| WebhookError::StorageError(e.to_string()))?;
        }

        let delivery_id = delivery.id;

        // Enqueue delivery for background processing
        if let Some(queue) = &self.delivery_queue {
            let task = DeliveryTask {
                webhook: webhook.clone(),
                payload: payload_value,
                delivery_id,
            };

            if let Err(e) = queue.enqueue(task).await {
                self.logger.error(
                    "webhook_engine",
                    "enqueue_failed",
                    &format!("Failed to enqueue webhook delivery: {}", e)
                );
                // Update delivery status to failed
                if let Ok(mut storage) = self.storage.lock() {
                    delivery.status = DeliveryStatus::Failed;
                    delivery.error_message = Some(format!("Failed to enqueue: {}", e));
                    let _ = storage.store_webhook_delivery(&delivery);
                }
            } else {
                self.logger.info(
                    "webhook_engine",
                    "delivery_enqueued",
                    &format!("Webhook delivery enqueued: {}", delivery_id)
                );
            }
        } else {
            self.logger.warn(
                "webhook_engine",
                "no_delivery_queue",
                "Webhook delivery queue not configured - delivery will not be processed"
            );
        }

        Ok(delivery_id)
    }

    /// Get delivery history for a circuit
    pub fn get_delivery_history(
        &self,
        circuit_id: &Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<WebhookDelivery>, WebhookError> {
        let storage = self.storage.lock()
            .map_err(|_| WebhookError::StorageError("Storage mutex poisoned".to_string()))?;
        storage.get_webhook_deliveries_by_circuit(circuit_id, limit)
            .map_err(|e| WebhookError::StorageError(e.to_string()))
    }

    /// Get delivery by ID
    pub fn get_delivery(
        &self,
        delivery_id: &Uuid,
    ) -> Result<Option<WebhookDelivery>, WebhookError> {
        let storage = self.storage.lock()
            .map_err(|_| WebhookError::StorageError("Storage mutex poisoned".to_string()))?;
        storage.get_webhook_delivery(delivery_id)
            .map_err(|e| WebhookError::StorageError(e.to_string()))
    }

    /// Validate webhook URL (basic validation to prevent SSRF)
    pub fn validate_webhook_url(url: &str) -> Result<(), WebhookError> {
        let parsed = url::Url::parse(url)
            .map_err(|e| WebhookError::ValidationError(format!("Invalid URL: {}", e)))?;

        // Only allow HTTP and HTTPS
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(WebhookError::ValidationError(
                "Only HTTP and HTTPS URLs are allowed".to_string(),
            ));
        }

        // Reject localhost and private IP ranges
        if let Some(host) = parsed.host_str() {
            if host == "localhost" || host == "127.0.0.1" || host == "0.0.0.0" {
                return Err(WebhookError::ValidationError(
                    "Localhost URLs are not allowed".to_string(),
                ));
            }

            // Check for private IP ranges (simple check)
            if host.starts_with("192.168.") || host.starts_with("10.") || host.starts_with("172.16.") {
                return Err(WebhookError::ValidationError(
                    "Private IP addresses are not allowed".to_string(),
                ));
            }
        }

        Ok(())
    }
}
