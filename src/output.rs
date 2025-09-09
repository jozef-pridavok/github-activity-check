use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::types::{CommitInfo, ReleaseInfo};
use crate::config::Config;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Default,
    Json,
    Field(String),
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Default => write!(f, "default"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Field(field) => write!(f, "field:{field}"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(OutputFormat::Default),
            "json" => Ok(OutputFormat::Json),
            s if s.starts_with("field:") => {
                let field = s.strip_prefix("field:").unwrap_or("");
                if field.is_empty() {
                    anyhow::bail!("Field name cannot be empty. Use format: field:field_name");
                }
                Ok(OutputFormat::Field(field.to_string()))
            }
            _ => anyhow::bail!("Invalid format '{}'. Use 'default', 'json', or 'field:field_name'", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryReport {
    pub owner: String,
    pub repo: String,
    pub commits_total: usize,
    pub contributors_total: usize,
    pub open_pull_requests: usize,
    pub open_issues: usize,
    pub last_commit: LastCommitInfo,
    pub last_release: Option<LastReleaseInfo>,
    pub project_alive: bool,
    pub criteria: CriteriaInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastCommitInfo {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    pub date_utc: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastReleaseInfo {
    pub tag_name: String,
    pub name: Option<String>,
    pub date_utc: Option<DateTime<Utc>>,
    pub is_prerelease: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriteriaInfo {
    pub max_days: i64,
    pub min_contributors: usize,
    pub min_commits: usize,
}

pub fn create_repository_report(
    config: &Config,
    commits_count: usize,
    contributors_count: usize,
    open_prs: usize,
    open_issues: usize,
    last_commit: &CommitInfo,
    last_release: Option<&ReleaseInfo>,
    alive: bool,
) -> RepositoryReport {
    RepositoryReport {
        owner: config.get_owner().to_string(),
        repo: config.get_repo().to_string(),
        commits_total: commits_count,
        contributors_total: contributors_count,
        open_pull_requests: open_prs,
        open_issues,
        last_commit: LastCommitInfo {
            sha: last_commit.sha.clone(),
            author_name: last_commit.commit.author.name.clone(),
            author_email: last_commit.commit.author.email.clone(),
            date_utc: last_commit.commit.author.date,
            message: first_line(&last_commit.commit.message).to_string(),
        },
        last_release: last_release.map(|release| LastReleaseInfo {
            tag_name: release.tag_name.clone(),
            name: release.name.clone(),
            date_utc: release.published_at,
            is_prerelease: release.prerelease,
        }),
        project_alive: alive,
        criteria: CriteriaInfo {
            max_days: config.get_max_days(),
            min_contributors: config.get_min_contributors(),
            min_commits: config.get_min_commits(),
        },
    }
}

pub fn print_output(
    config: &Config,
    report: &RepositoryReport,
) -> Result<()> {
    match config.get_format() {
        OutputFormat::Default => {
            print_default_output(config, report);
        }
        OutputFormat::Json => {
            print_json_output(report)?;
        }
        OutputFormat::Field(field_name) => {
            print_field_output(report, field_name)?;
        }
    }
    Ok(())
}

fn extract_field_value(report: &RepositoryReport, field_path: &str) -> Result<String> {
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
    
    // Convert the final value to string representation
    let result = match current {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => serde_json::to_string(other)?.trim_matches('"').to_string(),
    };
    
    Ok(result)
}

fn print_field_output(report: &RepositoryReport, field_name: &str) -> Result<()> {
    let value = extract_field_value(report, field_name)?;
    println!("{value}");
    Ok(())
}

fn print_default_output(config: &Config, report: &RepositoryReport) {
    println!("Repo: {}/{}", report.owner, report.repo);
    println!("-------------------------------------------");
    println!("Commits total            : {}", report.commits_total);
    println!("Contributors total       : {}", report.contributors_total);
    println!("Open pull requests       : {}", report.open_pull_requests);
    println!("Open issues (unresolved) : {}", report.open_issues);
    println!("Last commit              :");
    println!("  sha                    : {}", report.last_commit.sha);
    println!(
        "  author                 : {} <{}>",
        report.last_commit.author_name, report.last_commit.author_email
    );
    println!("  date (UTC)             : {}", report.last_commit.date_utc);
    println!("  message                : {}", report.last_commit.message);
    
    if let Some(ref release) = report.last_release {
        println!("Last release             :");
        println!("  tag                    : {}", release.tag_name);
        if let Some(ref name) = release.name {
            println!("  name                   : {}", name);
        }
        if let Some(date) = release.date_utc {
            let days_since_release = chrono::Utc::now().signed_duration_since(date).num_days();
            println!("  date (UTC)             : {}", date);
            
            let release_status = if days_since_release <= config.get_max_release_days() {
                if release.is_prerelease {
                    "Recent prerelease ⚡"
                } else {
                    "Fresh release ✅"
                }
            } else {
                if release.is_prerelease {
                    "Stale prerelease ⚠️"
                } else {
                    "Stale release ⚠️"
                }
            };
            println!("  status                 : {} ({} days ago)", release_status, days_since_release);
        } else {
            println!("  date (UTC)             : Not available");
            println!("  status                 : Unknown age ❓");
        }
        println!("  prerelease             : {}", if release.is_prerelease { "Yes" } else { "No" });
    } else {
        println!("Last release             : No releases found");
    }
    
    println!("-------------------------------------------");
    println!(
        "Project alive           : {}",
        if report.project_alive { "ALIVE ✅" } else { "LIKELY DEAD ⚠️" }
    );
    println!(
        "Criteria: last ≤ {} days or (contributors ≥ {} and commits ≥ {})",
        config.get_max_days(), config.get_min_contributors(), config.get_min_commits()
    );
}

fn print_json_output(report: &RepositoryReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    Ok(())
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert!(matches!(OutputFormat::from_str("default").unwrap(), OutputFormat::Default));
        assert!(matches!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json));
        
        if let OutputFormat::Field(field) = OutputFormat::from_str("field:commits_total").unwrap() {
            assert_eq!(field, "commits_total");
        } else {
            panic!("Expected Field variant");
        }
        
        assert!(OutputFormat::from_str("field:").is_err());
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Default.to_string(), "default");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Field("test".to_string()).to_string(), "field:test");
    }

    #[test]
    fn test_first_line() {
        assert_eq!(first_line("single line"), "single line");
        assert_eq!(first_line("first line\nsecond line"), "first line");
        assert_eq!(first_line(""), "");
        assert_eq!(first_line("line\n\n"), "line");
    }
}