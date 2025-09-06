use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::str::FromStr;

use crate::output::OutputFormat;

#[derive(Parser, Deserialize, Serialize, Debug, Clone, Default)]
#[command(name = "github-activity-check")]
#[command(about = "CLI tool to check if GitHub repositories are actively maintained")]
#[command(version)]
pub struct Config {
    /// Repository owner
    #[arg(value_name = "OWNER")]
    #[serde(skip)]
    pub owner: Option<String>,
    
    /// Repository name
    #[arg(value_name = "REPO")]
    #[serde(skip)]
    pub repo: Option<String>,
    
    /// Configuration file path
    #[arg(short, long)]
    #[serde(skip)]
    pub config_file: Option<String>,
    
    /// Output format
    #[arg(long, value_parser = OutputFormat::from_str)]
    #[serde(default)]
    pub format: Option<OutputFormat>,
    
    /// Minimum number of commits for established project
    #[arg(long)]
    #[serde(default)]
    pub min_commits: Option<usize>,
    
    /// Minimum number of contributors for established project
    #[arg(long)]
    #[serde(default)]
    pub min_contributors: Option<usize>,
    
    /// Maximum days since last commit for active project
    #[arg(long)]
    #[serde(default)]
    pub max_days: Option<i64>,
    
    /// Scale for open pull requests scoring
    #[arg(long)]
    #[serde(default)]
    pub prs_scale: Option<f64>,
    
    /// Scale for open issues scoring
    #[arg(long)]
    #[serde(default)]
    pub issues_scale: Option<f64>,
    
    /// History file path for storing last run data
    #[arg(long)]
    #[serde(skip)]
    pub history: Option<String>,
    
    /// Check for changes in specific field compared to history (exit code = change magnitude)
    #[arg(long)]
    #[serde(skip)]
    pub check: Option<String>,
    
    /// Enable verbose output (shows what the tool is doing)
    #[arg(long, default_value_t = false)]
    #[serde(skip)]
    pub verbose: bool,
}

impl Config {
    pub fn from_toml<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read configuration file")?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse TOML configuration file")?;
            
        Ok(config)
    }

    pub fn merge(mut self, file_config: Config) -> Self {
        // CLI has precedence, if None, take from file_config
        self.format = self.format.or(file_config.format);
        self.min_commits = self.min_commits.or(file_config.min_commits);
        self.min_contributors = self.min_contributors.or(file_config.min_contributors);
        self.max_days = self.max_days.or(file_config.max_days);
        self.prs_scale = self.prs_scale.or(file_config.prs_scale);
        self.issues_scale = self.issues_scale.or(file_config.issues_scale);
        self
    }

    pub fn with_defaults(mut self) -> Self {
        self.format = self.format.or(Some(OutputFormat::Default));
        self.min_commits = self.min_commits.or(Some(100));
        self.min_contributors = self.min_contributors.or(Some(3));
        self.max_days = self.max_days.or(Some(60));
        self.prs_scale = self.prs_scale.or(Some(10.0));
        self.issues_scale = self.issues_scale.or(Some(20.0));
        self
    }

    // Convenience getters that unwrap (safe after with_defaults)
    pub fn get_owner(&self) -> &str {
        self.owner.as_ref().expect("Owner should be set")
    }

    pub fn get_repo(&self) -> &str {
        self.repo.as_ref().expect("Repo should be set")
    }

    pub fn get_format(&self) -> &OutputFormat {
        self.format.as_ref().expect("Format should be set")
    }

    pub fn get_min_commits(&self) -> usize {
        self.min_commits.expect("min_commits should be set")
    }

    pub fn get_min_contributors(&self) -> usize {
        self.min_contributors.expect("min_contributors should be set")
    }

    pub fn get_max_days(&self) -> i64 {
        self.max_days.expect("max_days should be set")
    }

    pub fn get_prs_scale(&self) -> f64 {
        self.prs_scale.expect("prs_scale should be set")
    }

    pub fn get_issues_scale(&self) -> f64 {
        self.issues_scale.expect("issues_scale should be set")
    }

    pub fn validate(&self) -> Result<()> {
        if self.owner.is_none() {
            anyhow::bail!("Repository owner is required");
        }
        if self.repo.is_none() {
            anyhow::bail!("Repository name is required");
        }
        Ok(())
    }
}