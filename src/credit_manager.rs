use crate::storage::{StorageBackend, StorageError};
use crate::types::{CreditTransaction, CreditTransactionType, UserAccount, UserTier};
use chrono::{Datelike, Timelike, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreditEngine<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
}

impl<S: StorageBackend> CreditEngine<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        Self { storage }
    }

    pub async fn check_and_consume_credits(
        &self,
        user_id: &str,
        operation_type: &str,
        operation_id: &str,
    ) -> Result<bool, StorageError> {
        let cost = self.get_operation_cost(operation_type);

        let storage = self
            .storage
            .lock()
            .map_err(|_| StorageError::IoError("Credit manager Mutex poisoned".to_string()))?;

        // Get user account
        let mut user = match storage.get_user_account(user_id)? {
            Some(user) => user,
            None => return Ok(false), // User not found
        };

        // Check if user has enough credits
        if user.credits < cost {
            return Ok(false);
        }

        // Check tier limits
        if !self.check_tier_limits(&user, operation_type)? {
            return Ok(false);
        }

        // Consume credits
        user.credits -= cost;
        user.updated_at = Utc::now();

        // Record transaction
        let transaction = CreditTransaction {
            transaction_id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            amount: -cost,
            transaction_type: CreditTransactionType::Consumption,
            description: format!("Operation: {operation_type}"),
            operation_type: Some(operation_type.to_string()),
            operation_id: Some(operation_id.to_string()),
            timestamp: Utc::now(),
            balance_after: user.credits,
        };

        // Update user and record transaction
        storage.update_user_account(&user)?;
        storage.record_credit_transaction(&transaction)?;

        Ok(true)
    }

    pub async fn add_credits(
        &self,
        user_id: &str,
        amount: i64,
        description: &str,
    ) -> Result<(), StorageError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| StorageError::IoError("Credit manager Mutex poisoned".to_string()))?;

        let mut user = storage
            .get_user_account(user_id)?
            .ok_or(StorageError::NotFound)?;

        user.credits += amount;
        user.updated_at = Utc::now();

        let transaction = CreditTransaction {
            transaction_id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            amount,
            transaction_type: if amount >= 0 {
                CreditTransactionType::Grant
            } else {
                CreditTransactionType::Penalty
            },
            description: description.to_string(),
            operation_type: None,
            operation_id: None,
            timestamp: Utc::now(),
            balance_after: user.credits,
        };

        storage.update_user_account(&user)?;
        storage.record_credit_transaction(&transaction)?;

        Ok(())
    }

    pub async fn get_user_credit_balance(
        &self,
        user_id: &str,
    ) -> Result<Option<i64>, StorageError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| StorageError::IoError("Credit manager Mutex poisoned".to_string()))?;
        match storage.get_user_account(user_id)? {
            Some(user) => Ok(Some(user.credits)),
            None => Ok(None),
        }
    }

    pub async fn get_credit_history(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<CreditTransaction>, StorageError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| StorageError::IoError("Credit manager Mutex poisoned".to_string()))?;
        storage.get_credit_transactions(user_id, limit)
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn refund_operation(
        &self,
        user_id: &str,
        operation_id: &str,
        reason: &str,
    ) -> Result<bool, StorageError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| StorageError::IoError("Credit manager Mutex poisoned".to_string()))?;

        // Find the original transaction
        let transactions = storage.get_credit_transactions(user_id, None)?;
        let original_transaction = transactions.iter().find(|t| {
            t.operation_id.as_deref() == Some(operation_id)
                && t.transaction_type == CreditTransactionType::Consumption
        });

        if let Some(original) = original_transaction {
            drop(storage);
            // Refund the credits
            self.add_credits(
                user_id,
                -original.amount, // Refund the consumed amount (make it positive)
                &format!("Refund for operation {operation_id}: {reason}"),
            )
            .await?;
            return Ok(true);
        }

        Ok(false)
    }

    fn get_operation_cost(&self, operation_type: &str) -> i64 {
        match operation_type {
            "store_item" => 10,
            "store_event" => 5,
            "migrate_item" => 25,
            "circuit_execution" => 50,
            "bulk_export" => 100,
            "advanced_query" => 20,
            _ => 1, // Default cost
        }
    }

    fn check_tier_limits(
        &self,
        user: &UserAccount,
        operation_type: &str,
    ) -> Result<bool, StorageError> {
        // Check if operation is allowed for user's tier
        match (&user.tier, operation_type) {
            (UserTier::Basic, "circuit_execution") => Ok(false),
            (UserTier::Basic, "bulk_export") => Ok(false),
            (UserTier::Basic, "advanced_query") => Ok(false),
            (UserTier::Professional, "bulk_export") => Ok(true), // Professional tier allows bulk operations
            _ => Ok(true),
        }
    }

    pub async fn get_tier_costs(&self, tier: &UserTier) -> HashMap<String, i64> {
        let mut costs = HashMap::new();

        let multiplier = match tier {
            UserTier::Basic => 1.0,
            UserTier::Professional => 0.8,
            UserTier::Enterprise => 0.6,
            UserTier::Admin => 0.0, // Admin operations are free
        };

        costs.insert("store_item".to_string(), (10.0 * multiplier) as i64);
        costs.insert("store_event".to_string(), (5.0 * multiplier) as i64);
        costs.insert("migrate_item".to_string(), (25.0 * multiplier) as i64);
        costs.insert("circuit_execution".to_string(), (50.0 * multiplier) as i64);
        costs.insert("bulk_export".to_string(), (100.0 * multiplier) as i64);
        costs.insert("advanced_query".to_string(), (20.0 * multiplier) as i64);

        costs
    }

    pub async fn calculate_monthly_usage(
        &self,
        user_id: &str,
    ) -> Result<HashMap<String, i64>, StorageError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| StorageError::IoError("Credit manager Mutex poisoned".to_string()))?;
        let transactions = storage.get_credit_transactions(user_id, None)?;

        let start_of_month = chrono::Utc::now()
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();

        let mut usage = HashMap::new();

        for transaction in transactions {
            if transaction.timestamp >= start_of_month
                && transaction.transaction_type == CreditTransactionType::Consumption
            {
                if let Some(op_type) = &transaction.operation_type {
                    *usage.entry(op_type.clone()).or_insert(0) += -transaction.amount;
                }
            }
        }

        Ok(usage)
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn auto_refill_credits(&self, user_id: &str) -> Result<bool, StorageError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| StorageError::IoError("Credit manager Mutex poisoned".to_string()))?;
        let user = match storage.get_user_account(user_id)? {
            Some(user) => user,
            None => return Ok(false),
        };

        // Check if user has subscription and needs refill
        if let Some(subscription) = &user.subscription {
            if user.credits < 100 && subscription.auto_renew {
                drop(storage);

                let refill_amount = match user.tier {
                    UserTier::Basic => 1000,
                    UserTier::Professional => 5000,
                    UserTier::Enterprise => 25000,
                    UserTier::Admin => 100000,
                };

                self.add_credits(user_id, refill_amount, "Automatic subscription refill")
                    .await?;

                return Ok(true);
            }
        }

        Ok(false)
    }
}
