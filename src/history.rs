use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::output::RepositoryReport;

#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryData {
    pub last_data: RepositoryReport,
}

impl HistoryData {
    pub fn load<P: AsRef<Path>>(path: P, verbose: bool) -> Result<Option<Self>> {
        let path = path.as_ref();
        
        if verbose {
            eprintln!("[VERBOSE] Checking for history file: {}", path.display());
        }
        
        if !path.exists() {
            if verbose {
                eprintln!("[VERBOSE] History file does not exist: {}", path.display());
            }
            return Ok(None);
        }

        if verbose {
            eprintln!("[VERBOSE] Reading history file: {}", path.display());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read history file: {}", path.display()))?;
        
        if verbose {
            eprintln!("[VERBOSE] Parsing history file content ({} bytes)", content.len());
        }
        
        let history: HistoryData = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse history file: {}", path.display()))?;
            
        if verbose {
            eprintln!("[VERBOSE] Successfully loaded history data");
        }
            
        Ok(Some(history))
    }

    pub fn save<P: AsRef<Path>>(&self, path: P, verbose: bool) -> Result<()> {
        let path = path.as_ref();
        
        if verbose {
            eprintln!("[VERBOSE] Preparing to save history to: {}", path.display());
        }
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if verbose {
                eprintln!("[VERBOSE] Ensuring parent directory exists: {}", parent.display());
            }
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        if verbose {
            eprintln!("[VERBOSE] Serializing history data to JSON");
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize history data")?;
        
        if verbose {
            eprintln!("[VERBOSE] Writing {} bytes to history file", content.len());
        }
        
        fs::write(path, &content)
            .with_context(|| format!("Failed to write history file: {}", path.display()))?;
            
        if verbose {
            eprintln!("[VERBOSE] History file saved successfully: {}", path.display());
        }
            
        Ok(())
    }

    pub fn calculate_change(&self, current: &RepositoryReport, field_path: &str) -> Result<i64> {
        // Extract values from both current and last data
        let current_value = extract_field_value(current, field_path)?;
        let last_value = extract_field_value(&self.last_data, field_path)?;

        // Calculate change based on field type
        calculate_field_change(&current_value, &last_value, field_path)
    }
}

fn extract_field_value(report: &RepositoryReport, field_path: &str) -> Result<serde_json::Value> {
    // Convert report to JSON for flexible field extraction
    let json_value = serde_json::to_value(report)?;
    
    // Split field path by dots for nested access
    let path_parts: Vec<&str> = field_path.split('.').collect();
    
    // Navigate through the JSON structure
    let mut current = &json_value;
    for part in &path_parts {
        current = current.get(part)
            .ok_or_else(|| anyhow::anyhow!("Field '{}' not found in path '{}'", part, field_path))?;
    }
    
    Ok(current.clone())
}

fn calculate_field_change(current: &serde_json::Value, last: &serde_json::Value, field_path: &str) -> Result<i64> {
    use serde_json::Value;
    
    match (current, last) {
        // Numbers - return absolute difference
        (Value::Number(curr), Value::Number(last)) => {
            let curr_f64 = curr.as_f64().unwrap_or(0.0);
            let last_f64 = last.as_f64().unwrap_or(0.0);
            Ok((curr_f64 - last_f64).abs() as i64)
        }
        
        // Booleans - return 0 if same, 1 if different
        (Value::Bool(curr), Value::Bool(last)) => {
            Ok(if curr == last { 0 } else { 1 })
        }
        
        // Special handling for dates (if field name suggests it's a date)
        (Value::String(curr), Value::String(last)) if field_path.contains("date") => {
            // Try to parse as ISO 8601 datetime
            if let (Ok(curr_date), Ok(last_date)) = (
                curr.parse::<DateTime<Utc>>(),
                last.parse::<DateTime<Utc>>()
            ) {
                let diff = curr_date - last_date;
                Ok(diff.num_days().abs())
            } else {
                // Fall back to string comparison
                Ok(if curr == last { 0 } else { 1 })
            }
        }
        
        // Strings - return 0 if same, 1 if different
        (Value::String(curr), Value::String(last)) => {
            Ok(if curr == last { 0 } else { 1 })
        }
        
        // Null values
        (Value::Null, Value::Null) => Ok(0),
        (Value::Null, _) | (_, Value::Null) => Ok(1),
        
        // Different types - always consider as changed
        _ => Ok(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_calculate_field_change() {
        use serde_json::json;
        
        // Numbers
        assert_eq!(calculate_field_change(&json!(100), &json!(90), "commits").unwrap(), 10);
        assert_eq!(calculate_field_change(&json!(90), &json!(100), "commits").unwrap(), 10);
        
        // Booleans
        assert_eq!(calculate_field_change(&json!(true), &json!(true), "alive").unwrap(), 0);
        assert_eq!(calculate_field_change(&json!(true), &json!(false), "alive").unwrap(), 1);
        
        // Strings
        assert_eq!(calculate_field_change(&json!("same"), &json!("same"), "owner").unwrap(), 0);
        assert_eq!(calculate_field_change(&json!("diff"), &json!("other"), "owner").unwrap(), 1);
    }

    #[test]
    fn test_history_save_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_history.json");
        
        // Create test data
        use crate::output::{LastCommitInfo, CriteriaInfo};
        
        let report = RepositoryReport {
            owner: "test".to_string(),
            repo: "repo".to_string(),
            commits_total: 100,
            contributors_total: 10,
            open_pull_requests: 5,
            open_issues: 20,
            last_commit: LastCommitInfo {
                sha: "abc123".to_string(),
                author_name: "author".to_string(),
                author_email: "author@test.com".to_string(),
                date_utc: Utc::now(),
                message: "test commit".to_string(),
            },
            project_alive: true,
            criteria: CriteriaInfo {
                max_days: 60,
                min_contributors: 3,
                min_commits: 100,
            },
        };
        
        let history = HistoryData { last_data: report };
        
        // Save and load
        history.save(&file_path, false).unwrap();
        let loaded = HistoryData::load(&file_path, false).unwrap().unwrap();
        
        assert_eq!(loaded.last_data.owner, "test");
        assert_eq!(loaded.last_data.commits_total, 100);
    }
}