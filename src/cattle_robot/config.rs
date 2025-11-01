use std::env;

#[derive(Debug, Clone)]
pub struct RobotConfig {
    pub railway_api_url: String,
    pub robot_api_key: String,
    pub robot_circuit_id: Option<String>,
    pub database_url: String,
    pub mode: RobotMode,
    pub schedule: ScheduleMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotMode {
    Production,
    DryRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleMode {
    WeekdayHeavy, // 2-5/hr weekdays, ~1/hr weekends
    Uniform,      // Constant rate regardless of day/time
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingVar(String),

    #[error("Invalid value for {var}: {value}")]
    InvalidValue { var: String, value: String },
}

impl RobotConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let railway_api_url = env::var("RAILWAY_API_URL")
            .unwrap_or_else(|_| "https://defarm-engines-api-production.up.railway.app".to_string());

        let robot_api_key = env::var("ROBOT_API_KEY")
            .map_err(|_| ConfigError::MissingVar("ROBOT_API_KEY".to_string()))?;

        let robot_circuit_id = env::var("ROBOT_CIRCUIT_ID").ok();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingVar("DATABASE_URL".to_string()))?;

        let mode = match env::var("ROBOT_MODE").as_deref() {
            Ok("production") => RobotMode::Production,
            Ok("dry-run") | Ok("dryrun") => RobotMode::DryRun,
            Ok("") | Err(_) => RobotMode::Production, // Default to production
            Ok(val) => {
                return Err(ConfigError::InvalidValue {
                    var: "ROBOT_MODE".to_string(),
                    value: val.to_string(),
                })
            }
        };

        let schedule = match env::var("ROBOT_SCHEDULE").as_deref() {
            Ok("weekday-heavy") | Ok("weekday_heavy") => ScheduleMode::WeekdayHeavy,
            Ok("uniform") => ScheduleMode::Uniform,
            Ok("") | Err(_) => ScheduleMode::WeekdayHeavy, // Default
            Ok(val) => {
                return Err(ConfigError::InvalidValue {
                    var: "ROBOT_SCHEDULE".to_string(),
                    value: val.to_string(),
                })
            }
        };

        Ok(Self {
            railway_api_url,
            robot_api_key,
            robot_circuit_id,
            database_url,
            mode,
            schedule,
        })
    }

    pub fn is_dry_run(&self) -> bool {
        matches!(self.mode, RobotMode::DryRun)
    }

    pub fn has_circuit(&self) -> bool {
        self.robot_circuit_id.is_some()
    }

    pub fn circuit_id(&self) -> Option<&str> {
        self.robot_circuit_id.as_deref()
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.robot_api_key.is_empty() {
            return Err(ConfigError::MissingVar("ROBOT_API_KEY".to_string()));
        }

        if self.database_url.is_empty() {
            return Err(ConfigError::MissingVar("DATABASE_URL".to_string()));
        }

        if !self.railway_api_url.starts_with("http://")
            && !self.railway_api_url.starts_with("https://")
        {
            return Err(ConfigError::InvalidValue {
                var: "RAILWAY_API_URL".to_string(),
                value: self.railway_api_url.clone(),
            });
        }

        Ok(())
    }

    pub fn summary(&self) -> String {
        format!(
            "Robot Configuration:\n\
             - API URL: {}\n\
             - Mode: {:?}\n\
             - Schedule: {:?}\n\
             - Circuit ID: {}\n\
             - Database: {}",
            self.railway_api_url,
            self.mode,
            self.schedule,
            self.robot_circuit_id.as_deref().unwrap_or("NOT SET"),
            if self.database_url.contains("@") {
                let parts: Vec<&str> = self.database_url.split('@').collect();
                format!("{}@***", parts[0].split(':').next().unwrap_or("postgres"))
            } else {
                "***".to_string()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robot_mode_is_dry_run() {
        let mut config = RobotConfig {
            railway_api_url: "http://localhost:3000".to_string(),
            robot_api_key: "test_key".to_string(),
            robot_circuit_id: None,
            database_url: "postgres://localhost/test".to_string(),
            mode: RobotMode::DryRun,
            schedule: ScheduleMode::WeekdayHeavy,
        };

        assert!(config.is_dry_run());

        config.mode = RobotMode::Production;
        assert!(!config.is_dry_run());
    }

    #[test]
    fn test_has_circuit() {
        let mut config = RobotConfig {
            railway_api_url: "http://localhost:3000".to_string(),
            robot_api_key: "test_key".to_string(),
            robot_circuit_id: None,
            database_url: "postgres://localhost/test".to_string(),
            mode: RobotMode::Production,
            schedule: ScheduleMode::WeekdayHeavy,
        };

        assert!(!config.has_circuit());

        config.robot_circuit_id = Some("test-circuit-id".to_string());
        assert!(config.has_circuit());
        assert_eq!(config.circuit_id(), Some("test-circuit-id"));
    }

    #[test]
    fn test_validate() {
        let config = RobotConfig {
            railway_api_url: "https://api.example.com".to_string(),
            robot_api_key: "valid_key".to_string(),
            robot_circuit_id: Some("circuit-123".to_string()),
            database_url: "postgres://localhost/db".to_string(),
            mode: RobotMode::Production,
            schedule: ScheduleMode::WeekdayHeavy,
        };

        assert!(config.validate().is_ok());

        let invalid_config = RobotConfig {
            railway_api_url: "invalid-url".to_string(),
            robot_api_key: "key".to_string(),
            robot_circuit_id: None,
            database_url: "postgres://localhost/db".to_string(),
            mode: RobotMode::Production,
            schedule: ScheduleMode::Uniform,
        };

        assert!(invalid_config.validate().is_err());
    }
}
