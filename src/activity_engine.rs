use crate::postgres_persistence::PostgresPersistence;
use crate::storage::{StorageBackend, StorageError};
use crate::types::{
    UserActivity, UserActivityCategory, UserActivityFilters, UserActivityListResponse,
    UserActivityStats, UserActivityType, UserResourceType,
};
use chrono::{DateTime, Duration, Timelike, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub enum ActivityError {
    StorageError(StorageError),
    ValidationError(String),
    ProcessingError(String),
}

impl From<StorageError> for ActivityError {
    fn from(err: StorageError) -> Self {
        ActivityError::StorageError(err)
    }
}

impl std::fmt::Display for ActivityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityError::StorageError(e) => write!(f, "Storage error: {e}"),
            ActivityError::ValidationError(e) => write!(f, "Validation error: {e}"),
            ActivityError::ProcessingError(e) => write!(f, "Processing error: {e}"),
        }
    }
}

impl std::error::Error for ActivityError {}

#[derive(Clone)]
pub struct ActivityEngine<S: StorageBackend> {
    storage: S,
    postgres: Option<Arc<RwLock<Option<PostgresPersistence>>>>,
}

impl<S: StorageBackend + 'static> ActivityEngine<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            postgres: None,
        }
    }

    pub fn with_postgres(mut self, postgres: Arc<RwLock<Option<PostgresPersistence>>>) -> Self {
        self.postgres = Some(postgres);
        self
    }

    pub fn set_postgres(&mut self, postgres: Arc<RwLock<Option<PostgresPersistence>>>) {
        self.postgres = Some(postgres);
    }

    pub fn get_storage(&self) -> &S {
        &self.storage
    }

    /// Record a user activity
    pub fn record_activity(&self, activity: &UserActivity) -> Result<(), ActivityError> {
        self.storage.store_user_activity(activity)?;

        if let Some(pg_ref) = &self.postgres {
            let activity_clone = activity.clone();
            let pg = Arc::clone(pg_ref);
            tokio::spawn(async move {
                let pg_lock = pg.read().await;
                if let Some(pg_persistence) = &*pg_lock {
                    if let Err(e) = pg_persistence.persist_user_activity(&activity_clone).await {
                        tracing::warn!(
                            "Failed to persist user activity {}: {}",
                            activity_clone.activity_id,
                            e
                        );
                    }
                }
            });
        }

        Ok(())
    }

    /// Record a simple activity with minimal details
    #[allow(clippy::too_many_arguments)]
    pub fn record_simple(
        &self,
        user_id: String,
        workspace_id: String,
        activity_type: UserActivityType,
        category: UserActivityCategory,
        resource_type: UserResourceType,
        resource_id: String,
        action: String,
        description: String,
    ) -> Result<String, ActivityError> {
        let activity = UserActivity::new(
            user_id,
            workspace_id,
            activity_type,
            category,
            resource_type,
            resource_id,
            action,
            description,
        );

        let activity_id = activity.activity_id.clone();
        self.record_activity(&activity)?;
        Ok(activity_id)
    }

    /// Get activities with filtering and pagination
    pub fn list_activities(
        &self,
        filters: &UserActivityFilters,
    ) -> Result<UserActivityListResponse, ActivityError> {
        let mut all_activities = self.storage.list_user_activities()?;

        // Apply filters
        if let Some(ref category) = filters.category {
            all_activities.retain(|a| &a.category == category);
        }

        if let Some(ref activity_type) = filters.activity_type {
            all_activities.retain(|a| &a.activity_type == activity_type);
        }

        if let Some(ref resource_type) = filters.resource_type {
            all_activities.retain(|a| &a.resource_type == resource_type);
        }

        if let Some(ref user_id) = filters.user_id {
            all_activities.retain(|a| &a.user_id == user_id);
        }

        if let Some(start_date) = filters.start_date {
            all_activities.retain(|a| a.timestamp >= start_date);
        }

        if let Some(end_date) = filters.end_date {
            all_activities.retain(|a| a.timestamp <= end_date);
        }

        if let Some(ref search_query) = filters.search_query {
            let query_lower = search_query.to_lowercase();
            all_activities.retain(|a| {
                a.description.to_lowercase().contains(&query_lower)
                    || a.action.to_lowercase().contains(&query_lower)
                    || a.resource_id.to_lowercase().contains(&query_lower)
            });
        }

        // Sort by timestamp descending (newest first)
        all_activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let total = all_activities.len();
        let page = filters.page.unwrap_or(1);
        let per_page = filters.per_page.unwrap_or(50);
        let total_pages = total.div_ceil(per_page);

        // Paginate
        let start = (page - 1) * per_page;
        let end = std::cmp::min(start + per_page, total);
        let activities = all_activities[start..end].to_vec();

        Ok(UserActivityListResponse {
            activities,
            total,
            page,
            per_page,
            total_pages,
        })
    }

    /// Get a specific activity by ID
    pub fn get_activity(&self, activity_id: &str) -> Result<Option<UserActivity>, ActivityError> {
        let activities = self.storage.list_user_activities()?;
        Ok(activities
            .into_iter()
            .find(|a| a.activity_id == activity_id))
    }

    /// Get activity statistics for a given period
    pub fn get_stats(&self, period_days: i64) -> Result<UserActivityStats, ActivityError> {
        let now = Utc::now();
        let period_start = now - Duration::days(period_days);
        let period_end = now;

        let activities = self.storage.list_user_activities()?;

        // Filter to period
        let period_activities: Vec<_> = activities
            .into_iter()
            .filter(|a| a.timestamp >= period_start && a.timestamp <= period_end)
            .collect();

        let total_actions = period_activities.len();

        // Group by category
        let mut by_category: HashMap<String, usize> = HashMap::new();
        for activity in &period_activities {
            let category = format!("{:?}", activity.category);
            *by_category.entry(category).or_insert(0) += 1;
        }

        // Group by type
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for activity in &period_activities {
            let activity_type = format!("{:?}", activity.activity_type);
            *by_type.entry(activity_type).or_insert(0) += 1;
        }

        // Most active hours
        let mut hour_counts: HashMap<usize, usize> = HashMap::new();
        for activity in &period_activities {
            let hour = activity.timestamp.hour() as usize;
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        let mut most_active_hours: Vec<(usize, usize)> = hour_counts.into_iter().collect();
        most_active_hours.sort_by(|a, b| b.1.cmp(&a.1));
        most_active_hours.truncate(5);

        // Success rate
        let successful = period_activities.iter().filter(|a| a.success).count();
        let success_rate = if total_actions > 0 {
            (successful as f64 / total_actions as f64) * 100.0
        } else {
            100.0
        };

        Ok(UserActivityStats {
            total_actions,
            by_category,
            by_type,
            most_active_hours,
            success_rate,
            period_start,
            period_end,
        })
    }

    /// Delete activities older than a certain date (for cleanup/retention)
    pub fn cleanup_old_activities(
        &self,
        before_date: DateTime<Utc>,
    ) -> Result<usize, ActivityError> {
        let mut activities = self.storage.list_user_activities()?;
        let original_count = activities.len();

        activities.retain(|a| a.timestamp >= before_date);

        self.storage.clear_user_activities()?;
        for activity in activities {
            self.storage.store_user_activity(&activity)?;
        }

        let deleted_count = original_count - self.storage.list_user_activities()?.len();
        Ok(deleted_count)
    }
}
