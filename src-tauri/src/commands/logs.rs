use std::collections::VecDeque;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tauri::State;

use crate::app::AppManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LogStore {
    entries: Arc<RwLock<VecDeque<LogEntry>>>,
    max_entries: usize,
}

impl LogStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub fn log(&self, level: &str, message: &str) {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: message.to_string(),
        };

        let mut entries = self.entries.write();
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    pub fn get_entries(&self, filter: Option<&str>) -> Vec<LogEntry> {
        let entries = self.entries.read();
        match filter {
            Some(level) => entries
                .iter()
                .filter(|e| e.level == level)
                .cloned()
                .collect(),
            None => entries.iter().cloned().collect(),
        }
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[tauri::command]
pub async fn get_logs(
    manager: State<'_, AppManager>,
    filter: Option<String>,
) -> Result<String, String> {
    let store = manager.log_store();
    let entries = store.get_entries(filter.as_deref());
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_logs(
    manager: State<'_, AppManager>,
) -> Result<String, String> {
    manager.log_store().clear();
    Ok("cleared".to_string())
}
