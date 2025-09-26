use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub engine: String,
    pub event_type: String,
    pub message: String,
    pub context: HashMap<String, String>,
}

impl LogEntry {
    pub fn new(
        level: LogLevel,
        engine: impl Into<String>,
        event_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level,
            engine: engine.into(),
            event_type: event_type.into(),
            message: message.into(),
            context: HashMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

pub struct LoggingEngine {
    logs: Vec<LogEntry>,
}

impl LoggingEngine {
    pub fn new() -> Self {
        Self { logs: Vec::new() }
    }

    pub fn log(&mut self, entry: LogEntry) {
        self.logs.push(entry);
    }

    pub fn info(&mut self, engine: impl Into<String>, event_type: impl Into<String>, message: impl Into<String>) -> LogEntry {
        let entry = LogEntry::new(LogLevel::Info, engine, event_type, message);
        self.logs.push(entry.clone());
        entry
    }

    pub fn warn(&mut self, engine: impl Into<String>, event_type: impl Into<String>, message: impl Into<String>) -> LogEntry {
        let entry = LogEntry::new(LogLevel::Warn, engine, event_type, message);
        self.logs.push(entry.clone());
        entry
    }

    pub fn error(&mut self, engine: impl Into<String>, event_type: impl Into<String>, message: impl Into<String>) -> LogEntry {
        let entry = LogEntry::new(LogLevel::Error, engine, event_type, message);
        self.logs.push(entry.clone());
        entry
    }

    pub fn get_logs(&self) -> &[LogEntry] {
        &self.logs
    }

    pub fn get_logs_by_engine(&self, engine: &str) -> Vec<&LogEntry> {
        self.logs.iter().filter(|log| log.engine == engine).collect()
    }

    pub fn get_logs_by_level(&self, level: &LogLevel) -> Vec<&LogEntry> {
        self.logs.iter().filter(|log| std::mem::discriminant(&log.level) == std::mem::discriminant(level)).collect()
    }

    pub fn get_logs_by_event_type(&self, event_type: &str) -> Vec<&LogEntry> {
        self.logs.iter().filter(|log| log.event_type == event_type).collect()
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }
}

impl Default for LoggingEngine {
    fn default() -> Self {
        Self::new()
    }
}