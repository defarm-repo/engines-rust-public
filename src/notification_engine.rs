use crate::storage::StorageBackend;
use crate::types::{Notification, NotificationType};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug)]
pub enum NotificationError {
    StorageError(String),
    NotFound,
    ValidationError(String),
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationError::StorageError(e) => write!(f, "Storage error: {}", e),
            NotificationError::NotFound => write!(f, "Notification not found"),
            NotificationError::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for NotificationError {}

pub struct NotificationEngine<S: StorageBackend> {
    storage: Arc<std::sync::Mutex<S>>,
}

impl<S: StorageBackend> NotificationEngine<S> {
    pub fn new(storage: Arc<std::sync::Mutex<S>>) -> Self {
        Self { storage }
    }

    /// Create a notification for when a user requests to join a circuit
    pub fn create_join_request_notification(
        &self,
        admin_user_id: &str,
        requester_id: &str,
        circuit_id: &str,
        circuit_name: &str,
        message: Option<&str>,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            admin_user_id.to_string(),
            NotificationType::JoinRequestReceived,
            format!("New join request for {}", circuit_name),
            format!("User {} requested to join your circuit", requester_id),
            json!({
                "requester_id": requester_id,
                "circuit_id": circuit_id,
                "circuit_name": circuit_name,
                "message": message,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when a join request is approved
    pub fn create_join_approved_notification(
        &self,
        requester_id: &str,
        circuit_id: &str,
        circuit_name: &str,
        approved_by: &str,
        assigned_role: &str,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            requester_id.to_string(),
            NotificationType::JoinRequestApproved,
            format!("Join request approved for {}", circuit_name),
            format!(
                "Your request to join {} has been approved. You are now a {}.",
                circuit_name, assigned_role
            ),
            json!({
                "circuit_id": circuit_id,
                "circuit_name": circuit_name,
                "approved_by": approved_by,
                "assigned_role": assigned_role,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when a join request is rejected
    pub fn create_join_rejected_notification(
        &self,
        requester_id: &str,
        circuit_id: &str,
        circuit_name: &str,
        rejected_by: &str,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            requester_id.to_string(),
            NotificationType::JoinRequestRejected,
            format!("Join request rejected for {}", circuit_name),
            format!("Your request to join {} has been rejected.", circuit_name),
            json!({
                "circuit_id": circuit_id,
                "circuit_name": circuit_name,
                "rejected_by": rejected_by,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when a user is directly invited to a circuit
    pub fn create_circuit_invite_notification(
        &self,
        invited_user_id: &str,
        circuit_id: &str,
        circuit_name: &str,
        invited_by: &str,
        role: &str,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            invited_user_id.to_string(),
            NotificationType::CircuitInvite,
            format!("Invited to {}", circuit_name),
            format!(
                "You have been invited to join {} as a {}.",
                circuit_name, role
            ),
            json!({
                "circuit_id": circuit_id,
                "circuit_name": circuit_name,
                "invited_by": invited_by,
                "role": role,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when an item is shared to a circuit
    pub fn create_item_shared_notification(
        &self,
        member_user_id: &str,
        item_id: &str,
        circuit_id: &str,
        circuit_name: &str,
        shared_by: &str,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            member_user_id.to_string(),
            NotificationType::ItemShared,
            format!("New item in {}", circuit_name),
            format!("User {} shared a new item to {}.", shared_by, circuit_name),
            json!({
                "item_id": item_id,
                "circuit_id": circuit_id,
                "circuit_name": circuit_name,
                "shared_by": shared_by,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when an admin updates a user's account
    pub fn create_account_updated_notification(
        &self,
        user_id: &str,
        admin_username: &str,
        changes: &str,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            user_id.to_string(),
            NotificationType::AccountUpdated,
            "Account Updated".to_string(),
            format!(
                "Your account has been updated by admin {}. Changes: {}",
                admin_username, changes
            ),
            json!({
                "admin_username": admin_username,
                "changes": changes,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when an admin adjusts a user's credits
    pub fn create_credits_adjusted_notification(
        &self,
        user_id: &str,
        admin_username: &str,
        amount: i64,
        reason: &str,
        new_balance: i64,
    ) -> Result<Notification, NotificationError> {
        let action = if amount > 0 { "added" } else { "deducted" };
        let notification = Notification::new(
            user_id.to_string(),
            NotificationType::CreditsAdjusted,
            "Credits Adjusted".to_string(),
            format!(
                "Admin {} {} {} credits. Reason: {}. New balance: {}",
                admin_username,
                action,
                amount.abs(),
                reason,
                new_balance
            ),
            json!({
                "admin_username": admin_username,
                "amount": amount,
                "reason": reason,
                "new_balance": new_balance,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when an admin freezes/suspends a user's account
    pub fn create_account_frozen_notification(
        &self,
        user_id: &str,
        admin_username: &str,
        reason: &str,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            user_id.to_string(),
            NotificationType::AccountFrozen,
            "Account Frozen".to_string(),
            format!(
                "Your account has been frozen by admin {}. Reason: {}",
                admin_username, reason
            ),
            json!({
                "admin_username": admin_username,
                "reason": reason,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Create a notification for when an admin unfreezes/reactivates a user's account
    pub fn create_account_unfrozen_notification(
        &self,
        user_id: &str,
        admin_username: &str,
    ) -> Result<Notification, NotificationError> {
        let notification = Notification::new(
            user_id.to_string(),
            NotificationType::AccountUnfrozen,
            "Account Unfrozen".to_string(),
            format!(
                "Your account has been reactivated by admin {}. You can now access all features.",
                admin_username
            ),
            json!({
                "admin_username": admin_username,
                "timestamp": Utc::now().timestamp(),
            }),
        );

        self.store_notification(&notification)?;
        Ok(notification)
    }

    /// Get all notifications for a user
    pub fn get_user_notifications(
        &self,
        user_id: &str,
        since: Option<DateTime<Utc>>,
        limit: Option<usize>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| NotificationError::StorageError("Storage mutex poisoned".to_string()))?;
        storage
            .get_user_notifications(user_id, since, limit, unread_only)
            .map_err(|e| NotificationError::StorageError(e.to_string()))
    }

    /// Get a specific notification
    pub fn get_notification(
        &self,
        notification_id: &str,
    ) -> Result<Option<Notification>, NotificationError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| NotificationError::StorageError("Storage mutex poisoned".to_string()))?;
        storage
            .get_notification(notification_id)
            .map_err(|e| NotificationError::StorageError(e.to_string()))
    }

    /// Mark a notification as read
    pub fn mark_as_read(
        &self,
        notification_id: &str,
        user_id: &str,
    ) -> Result<Notification, NotificationError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| NotificationError::StorageError("Storage mutex poisoned".to_string()))?;

        let mut notification = storage
            .get_notification(notification_id)
            .map_err(|e| NotificationError::StorageError(e.to_string()))?
            .ok_or(NotificationError::NotFound)?;

        // Verify the notification belongs to this user
        if notification.user_id != user_id {
            return Err(NotificationError::ValidationError(
                "Notification does not belong to this user".to_string(),
            ));
        }

        notification.mark_as_read();
        storage
            .update_notification(&notification)
            .map_err(|e| NotificationError::StorageError(e.to_string()))?;

        Ok(notification)
    }

    /// Mark all notifications as read for a user
    pub fn mark_all_as_read(&self, user_id: &str) -> Result<usize, NotificationError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| NotificationError::StorageError("Storage mutex poisoned".to_string()))?;
        storage
            .mark_all_notifications_read(user_id)
            .map_err(|e| NotificationError::StorageError(e.to_string()))
    }

    /// Delete a notification
    pub fn delete_notification(
        &self,
        notification_id: &str,
        user_id: &str,
    ) -> Result<(), NotificationError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| NotificationError::StorageError("Storage mutex poisoned".to_string()))?;

        let notification = storage
            .get_notification(notification_id)
            .map_err(|e| NotificationError::StorageError(e.to_string()))?
            .ok_or(NotificationError::NotFound)?;

        // Verify the notification belongs to this user
        if notification.user_id != user_id {
            return Err(NotificationError::ValidationError(
                "Notification does not belong to this user".to_string(),
            ));
        }

        storage
            .delete_notification(notification_id)
            .map_err(|e| NotificationError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Get count of unread notifications for a user
    pub fn get_unread_count(&self, user_id: &str) -> Result<usize, NotificationError> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| NotificationError::StorageError("Storage mutex poisoned".to_string()))?;
        storage
            .get_unread_notification_count(user_id)
            .map_err(|e| NotificationError::StorageError(e.to_string()))
    }

    // Internal helper to store a notification
    fn store_notification(&self, notification: &Notification) -> Result<(), NotificationError> {
        let mut storage = self
            .storage
            .lock()
            .map_err(|_| NotificationError::StorageError("Storage mutex poisoned".to_string()))?;
        storage
            .store_notification(notification)
            .map_err(|e| NotificationError::StorageError(e.to_string()))?;
        Ok(())
    }
}
