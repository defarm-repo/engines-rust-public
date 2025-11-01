use chrono::{Datelike, Timelike, Utc, Weekday};
use rand::prelude::*;
use rand_distr::{Distribution, Exp};
use std::time::Duration as StdDuration;

/// Scheduler for cattle robot operations with realistic timing patterns
/// - Weekdays: 2-5 operations/hour (higher activity 9am-5pm BRT)
/// - Weekends: ~1 operation/hour
/// - Random jitter: ±20% variance to avoid detection
/// - Business hours emphasis: 60% of operations during 9am-5pm
pub struct CattleScheduler {
    rng: ThreadRng,
}

impl Default for CattleScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CattleScheduler {
    pub fn new() -> Self {
        Self { rng: thread_rng() }
    }

    /// Calculate next operation delay based on current time and day
    pub fn next_operation_delay(&mut self) -> StdDuration {
        let now = Utc::now();
        let weekday = now.weekday();
        let hour = now.hour();

        // Base rates (operations per hour)
        let base_rate = if self.is_weekend(weekday) {
            1.0 // Weekend: ~1 op/hour
        } else if self.is_business_hours(hour) {
            4.0 // Weekday business hours: ~4 ops/hour
        } else {
            2.0 // Weekday off-hours: ~2 ops/hour
        };

        // Add jitter ±20%
        let jitter = self.rng.gen_range(0.8..1.2);
        let adjusted_rate = base_rate * jitter;

        // Convert rate to mean interval (seconds)
        let mean_interval_secs = 3600.0 / adjusted_rate;

        // Use exponential distribution (Poisson process)
        let exp = Exp::new(1.0 / mean_interval_secs).unwrap();
        let delay_secs: f64 = exp.sample(&mut self.rng);

        // Clamp to reasonable bounds (5 min to 2 hours)
        let clamped_secs = delay_secs.clamp(300.0_f64, 7200.0_f64);

        StdDuration::from_secs_f64(clamped_secs)
    }

    /// Check if current time is weekend
    fn is_weekend(&self, weekday: Weekday) -> bool {
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    }

    /// Check if hour is business hours (9am-5pm)
    fn is_business_hours(&self, hour: u32) -> bool {
        (9..17).contains(&hour)
    }

    /// Select operation type (new mint vs update)
    /// - 70% new mints
    /// - 30% updates
    pub fn select_operation_type(&mut self) -> OperationType {
        if self.rng.gen_bool(0.7) {
            OperationType::NewMint
        } else {
            OperationType::Update
        }
    }

    /// Get current time info for logging
    pub fn current_time_info(&self) -> TimeInfo {
        let now = Utc::now();
        let weekday = now.weekday();
        let hour = now.hour();

        TimeInfo {
            timestamp: now,
            is_weekend: self.is_weekend(weekday),
            is_business_hours: self.is_business_hours(hour),
            weekday,
            hour,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    NewMint,
    Update,
}

#[derive(Debug, Clone)]
pub struct TimeInfo {
    pub timestamp: chrono::DateTime<Utc>,
    pub is_weekend: bool,
    pub is_business_hours: bool,
    pub weekday: Weekday,
    pub hour: u32,
}

impl TimeInfo {
    pub fn summary(&self) -> String {
        let day_type = if self.is_weekend {
            "Weekend"
        } else {
            "Weekday"
        };
        let hours_type = if self.is_business_hours {
            "Business hours"
        } else {
            "Off hours"
        };
        format!(
            "{} - {} {} ({})",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            day_type,
            hours_type,
            self.weekday
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_distribution() {
        let mut scheduler = CattleScheduler::new();
        let mut new_mints = 0;
        let mut updates = 0;

        for _ in 0..1000 {
            match scheduler.select_operation_type() {
                OperationType::NewMint => new_mints += 1,
                OperationType::Update => updates += 1,
            }
        }

        // Should be roughly 70/30 distribution
        let new_mint_ratio = new_mints as f64 / 1000.0;
        assert!(new_mint_ratio > 0.65 && new_mint_ratio < 0.75);
    }

    #[test]
    fn test_next_operation_delay() {
        let mut scheduler = CattleScheduler::new();

        for _ in 0..100 {
            let delay = scheduler.next_operation_delay();
            let delay_secs = delay.as_secs();

            // Should be between 5 min and 2 hours
            assert!(delay_secs >= 300);
            assert!(delay_secs <= 7200);
        }
    }

    #[test]
    fn test_business_hours_detection() {
        let scheduler = CattleScheduler::new();

        assert!(scheduler.is_business_hours(9));
        assert!(scheduler.is_business_hours(12));
        assert!(scheduler.is_business_hours(16));
        assert!(!scheduler.is_business_hours(8));
        assert!(!scheduler.is_business_hours(17));
        assert!(!scheduler.is_business_hours(22));
    }

    #[test]
    fn test_weekend_detection() {
        let scheduler = CattleScheduler::new();

        assert!(scheduler.is_weekend(Weekday::Sat));
        assert!(scheduler.is_weekend(Weekday::Sun));
        assert!(!scheduler.is_weekend(Weekday::Mon));
        assert!(!scheduler.is_weekend(Weekday::Fri));
    }

    #[test]
    fn test_time_info() {
        let scheduler = CattleScheduler::new();
        let info = scheduler.current_time_info();
        let summary = info.summary();

        assert!(!summary.is_empty());
        assert!(summary.contains("UTC"));
    }
}
