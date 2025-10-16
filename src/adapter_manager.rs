use crate::logging::LoggingEngine;
use crate::storage::StorageBackend;
use crate::types::{
    AdapterConfig, AdapterConnectionDetails, AdapterTestResult, AdapterType, AuthType,
    ConnectionTestResult, ContractConfigs, ContractInfo, ContractTestResult, TestStatus,
};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct AdapterManager<S: StorageBackend> {
    storage: Arc<Mutex<S>>,
    logger: std::cell::RefCell<LoggingEngine>,
}

impl<S: StorageBackend> AdapterManager<S> {
    pub fn new(storage: Arc<Mutex<S>>, logger: LoggingEngine) -> Self {
        Self {
            storage,
            logger: std::cell::RefCell::new(logger),
        }
    }

    /// Create a new adapter configuration
    pub fn create_adapter_config(
        &mut self,
        name: String,
        description: String,
        adapter_type: AdapterType,
        connection_details: AdapterConnectionDetails,
        contract_configs: Option<ContractConfigs>,
        created_by: String,
    ) -> Result<AdapterConfig, AdapterManagerError> {
        // Validate name uniqueness
        let existing_configs = self
            .storage
            .lock()
            .unwrap()
            .list_adapter_configs()
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?;

        if existing_configs.iter().any(|c| c.name == name) {
            return Err(AdapterManagerError::DuplicateName(name));
        }

        // Validate connection details
        self.validate_connection_details(&connection_details)?;

        // Validate contract configs if provided
        if let Some(ref contracts) = contract_configs {
            self.validate_contract_configs(contracts)?;
        }

        let mut config = AdapterConfig::new(
            name,
            description,
            adapter_type,
            connection_details,
            created_by.clone(),
        );

        config.contract_configs = contract_configs;

        // Store the configuration
        self.storage
            .lock()
            .unwrap()
            .store_adapter_config(&config)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?;

        self.logger
            .borrow_mut()
            .info(
                "adapter_manager",
                "adapter_created",
                "New adapter configuration created",
            )
            .with_context("config_id", config.config_id.to_string())
            .with_context("name", config.name.clone())
            .with_context("type", format!("{:?}", config.adapter_type))
            .with_context("created_by", created_by);

        Ok(config)
    }

    /// Update an existing adapter configuration
    pub fn update_adapter_config(
        &mut self,
        config_id: &Uuid,
        name: Option<String>,
        description: Option<String>,
        connection_details: Option<AdapterConnectionDetails>,
        contract_configs: Option<ContractConfigs>,
        is_active: Option<bool>,
    ) -> Result<AdapterConfig, AdapterManagerError> {
        let mut config = self
            .storage
            .lock()
            .unwrap()
            .get_adapter_config(config_id)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?
            .ok_or(AdapterManagerError::NotFound)?;

        // Update fields if provided
        if let Some(name) = name {
            // Check name uniqueness
            let existing_configs = self
                .storage
                .lock()
                .unwrap()
                .list_adapter_configs()
                .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?;

            if existing_configs
                .iter()
                .any(|c| c.name == name && c.config_id != *config_id)
            {
                return Err(AdapterManagerError::DuplicateName(name));
            }
            config.name = name;
        }

        if let Some(description) = description {
            config.description = description;
        }

        if let Some(details) = connection_details {
            self.validate_connection_details(&details)?;
            config.connection_details = details;
        }

        if let Some(contracts) = contract_configs {
            self.validate_contract_configs(&contracts)?;
            config.contract_configs = Some(contracts);
        }

        if let Some(active) = is_active {
            config.is_active = active;
        }

        // Save updated config
        self.storage
            .lock()
            .unwrap()
            .update_adapter_config(&config)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?;

        self.logger
            .borrow_mut()
            .info(
                "adapter_manager",
                "adapter_updated",
                "Adapter configuration updated",
            )
            .with_context("config_id", config_id.to_string())
            .with_context("name", config.name.clone());

        Ok(config)
    }

    /// Delete an adapter configuration
    pub fn delete_adapter_config(&mut self, config_id: &Uuid) -> Result<(), AdapterManagerError> {
        // Check if it exists
        let config = self
            .storage
            .lock()
            .unwrap()
            .get_adapter_config(config_id)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?
            .ok_or(AdapterManagerError::NotFound)?;

        // Don't allow deleting the default adapter
        if config.is_default {
            return Err(AdapterManagerError::CannotDeleteDefault);
        }

        self.storage
            .lock()
            .unwrap()
            .delete_adapter_config(config_id)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?;

        self.logger
            .borrow_mut()
            .info(
                "adapter_manager",
                "adapter_deleted",
                "Adapter configuration deleted",
            )
            .with_context("config_id", config_id.to_string())
            .with_context("name", config.name);

        Ok(())
    }

    /// Get adapter configuration by ID
    pub fn get_adapter_config(
        &self,
        config_id: &Uuid,
    ) -> Result<AdapterConfig, AdapterManagerError> {
        self.storage
            .lock()
            .unwrap()
            .get_adapter_config(config_id)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?
            .ok_or(AdapterManagerError::NotFound)
    }

    /// List all adapter configurations
    pub fn list_adapters(
        &self,
        active_only: bool,
    ) -> Result<Vec<AdapterConfig>, AdapterManagerError> {
        if active_only {
            self.storage
                .lock()
                .unwrap()
                .list_active_adapter_configs()
                .map_err(|e| AdapterManagerError::StorageError(e.to_string()))
        } else {
            self.storage
                .lock()
                .unwrap()
                .list_adapter_configs()
                .map_err(|e| AdapterManagerError::StorageError(e.to_string()))
        }
    }

    /// Set an adapter as the default
    pub fn set_default_adapter(&mut self, config_id: &Uuid) -> Result<(), AdapterManagerError> {
        self.storage
            .lock()
            .unwrap()
            .set_default_adapter(config_id)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?;

        self.logger
            .borrow_mut()
            .info(
                "adapter_manager",
                "default_adapter_set",
                "Default adapter configuration changed",
            )
            .with_context("config_id", config_id.to_string());

        Ok(())
    }

    /// Test an adapter configuration
    pub async fn test_adapter(
        &mut self,
        config_id: &Uuid,
    ) -> Result<AdapterTestResult, AdapterManagerError> {
        let config = self.get_adapter_config(config_id)?;

        let start_time = std::time::Instant::now();

        // Test connection
        let connection_test = self.test_connection(&config).await?;

        // Test contracts if configured
        let mut contract_tests = Vec::new();
        if let Some(ref contract_configs) = config.contract_configs {
            if let Some(ref mint_contract) = contract_configs.mint_contract {
                let test = self.test_contract(mint_contract, "mint").await?;
                contract_tests.push(test);
            }

            if let Some(ref ipcm_contract) = contract_configs.ipcm_contract {
                let test = self.test_contract(ipcm_contract, "ipcm").await?;
                contract_tests.push(test);
            }
        }

        let latency_ms = start_time.elapsed().as_millis() as u64;

        // Determine overall status
        let status = if !connection_test.success {
            TestStatus::Failed
        } else if contract_tests.iter().any(|t| !t.is_valid) {
            TestStatus::Failed
        } else if contract_tests.is_empty() && connection_test.success {
            TestStatus::Warning // No contracts to test
        } else {
            TestStatus::Passed
        };

        let result = AdapterTestResult {
            config_id: *config_id,
            tested_at: Utc::now(),
            status: status.clone(),
            connection_test,
            contract_tests,
            error_message: None,
            latency_ms: Some(latency_ms),
        };

        // Store test result
        self.storage
            .lock()
            .unwrap()
            .store_adapter_test_result(&result)
            .map_err(|e| AdapterManagerError::StorageError(e.to_string()))?;

        self.logger
            .borrow_mut()
            .info(
                "adapter_manager",
                "adapter_tested",
                "Adapter configuration tested",
            )
            .with_context("config_id", config_id.to_string())
            .with_context("status", format!("{status:?}"))
            .with_context("latency_ms", latency_ms.to_string());

        Ok(result)
    }

    // Private helper methods

    fn validate_connection_details(
        &self,
        details: &AdapterConnectionDetails,
    ) -> Result<(), AdapterManagerError> {
        if details.endpoint.is_empty() {
            return Err(AdapterManagerError::ValidationError(
                "Endpoint cannot be empty".to_string(),
            ));
        }

        if details.timeout_ms == 0 {
            return Err(AdapterManagerError::ValidationError(
                "Timeout must be greater than 0".to_string(),
            ));
        }

        if details.retry_attempts > 10 {
            return Err(AdapterManagerError::ValidationError(
                "Retry attempts cannot exceed 10".to_string(),
            ));
        }

        // Validate authentication based on auth_type
        match &details.auth_type {
            AuthType::ApiKey => {
                if details.api_key.is_none() {
                    return Err(AdapterManagerError::ValidationError(
                        "API key required for ApiKey auth type".to_string(),
                    ));
                }
            }
            AuthType::Bearer => {
                if details.api_key.is_none() {
                    return Err(AdapterManagerError::ValidationError(
                        "Bearer token required for Bearer auth type".to_string(),
                    ));
                }
            }
            AuthType::BasicAuth => {
                if details.api_key.is_none() || details.secret_key.is_none() {
                    return Err(AdapterManagerError::ValidationError(
                        "Username and password required for BasicAuth".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn validate_contract_configs(
        &self,
        configs: &ContractConfigs,
    ) -> Result<(), AdapterManagerError> {
        if configs.network.is_empty() {
            return Err(AdapterManagerError::ValidationError(
                "Network cannot be empty".to_string(),
            ));
        }

        if let Some(ref contract) = configs.mint_contract {
            if contract.contract_address.is_empty() {
                return Err(AdapterManagerError::ValidationError(
                    "Mint contract address cannot be empty".to_string(),
                ));
            }
        }

        if let Some(ref contract) = configs.ipcm_contract {
            if contract.contract_address.is_empty() {
                return Err(AdapterManagerError::ValidationError(
                    "IPCM contract address cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }

    async fn test_connection(
        &self,
        config: &AdapterConfig,
    ) -> Result<ConnectionTestResult, AdapterManagerError> {
        // Basic connectivity test
        // In a real implementation, this would make actual HTTP/network requests
        let start_time = std::time::Instant::now();

        // Simulate connection test
        let endpoint_reachable = !config.connection_details.endpoint.is_empty();
        let authentication_valid = match config.connection_details.auth_type {
            AuthType::None => true,
            AuthType::ApiKey => config.connection_details.api_key.is_some(),
            AuthType::Bearer => config.connection_details.api_key.is_some(),
            AuthType::BasicAuth => {
                config.connection_details.api_key.is_some()
                    && config.connection_details.secret_key.is_some()
            }
            _ => true,
        };

        let latency_ms = start_time.elapsed().as_millis() as u64;
        let success = endpoint_reachable && authentication_valid;

        Ok(ConnectionTestResult {
            success,
            endpoint_reachable,
            authentication_valid,
            latency_ms,
            error: if !success {
                Some("Connection test failed".to_string())
            } else {
                None
            },
        })
    }

    async fn test_contract(
        &self,
        contract: &ContractInfo,
        contract_type: &str,
    ) -> Result<ContractTestResult, AdapterManagerError> {
        // Validate contract configuration
        // In a real implementation, this would verify the contract on-chain

        let is_valid = !contract.contract_address.is_empty();
        let methods_verified: Vec<String> = contract.methods.keys().cloned().collect();

        Ok(ContractTestResult {
            contract_type: contract_type.to_string(),
            contract_address: contract.contract_address.clone(),
            is_valid,
            methods_verified,
            error: if !is_valid {
                Some("Invalid contract configuration".to_string())
            } else {
                None
            },
        })
    }
}

#[derive(Debug)]
pub enum AdapterManagerError {
    StorageError(String),
    NotFound,
    DuplicateName(String),
    ValidationError(String),
    CannotDeleteDefault,
    TestFailed(String),
}

impl std::fmt::Display for AdapterManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterManagerError::StorageError(msg) => write!(f, "Storage error: {msg}"),
            AdapterManagerError::NotFound => write!(f, "Adapter configuration not found"),
            AdapterManagerError::DuplicateName(name) => {
                write!(f, "Adapter with name '{name}' already exists")
            }
            AdapterManagerError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            AdapterManagerError::CannotDeleteDefault => {
                write!(f, "Cannot delete the default adapter")
            }
            AdapterManagerError::TestFailed(msg) => write!(f, "Adapter test failed: {msg}"),
        }
    }
}

impl std::error::Error for AdapterManagerError {}
