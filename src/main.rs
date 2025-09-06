use anyhow::Result;
use clap::Parser;

mod config;
mod github;
mod history;
mod output;
mod scoring;
mod types;

use config::Config;
use github::GitHubClient;
use history::HistoryData;
use output::{create_repository_report, print_output};
use scoring::ProjectScorer;

macro_rules! verbose_println {
    ($config:expr, $($arg:tt)*) => {
        if $config.verbose {
            eprintln!("[VERBOSE] {}", format!($($arg)*));
        }
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up error handling that always prints to stderr
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        
        // Print error chain if available
        let chain = e.chain().skip(1);
        for cause in chain {
            eprintln!("  Caused by: {cause}");
        }
        
        std::process::exit(1);
    }
    
    Ok(())
}

async fn run() -> Result<()> {
    let config = Config::parse();
    config.validate()?;

    // Load and merge configuration file if specified
    let config = if let Some(config_path) = &config.config_file {
        verbose_println!(&config, "Loading configuration file: {}", config_path);
        let file_config = Config::from_toml(config_path)?;
        config.merge(file_config).with_defaults()
    } else {
        config.with_defaults()
    };

    let token = std::env::var("GITHUB_TOKEN").ok();
    let github_client = GitHubClient::new(token.as_deref())?;
    let scorer = ProjectScorer::new();

    verbose_println!(&config, "Fetching repository data from GitHub API...");
    
    let last_commit = github_client.get_last_commit(config.get_owner(), config.get_repo()).await?;
    let commits_count = github_client.get_commit_count(config.get_owner(), config.get_repo()).await?;
    let contributors_count = github_client.get_contributors_count(config.get_owner(), config.get_repo()).await?;
    let open_prs = github_client.get_open_prs_count(config.get_owner(), config.get_repo()).await?;
    let open_issues = github_client.get_open_issues_count(config.get_owner(), config.get_repo()).await?;

    let alive = scorer.is_project_alive(
        &last_commit.commit.author.date,
        commits_count,
        contributors_count,
        open_prs,
        open_issues,
        &config,
    );

    let current_report = create_repository_report(
        &config,
        commits_count,
        contributors_count,
        open_prs,
        open_issues,
        &last_commit,
        alive,
    );

    // Handle history and check logic
    if let Some(history_path) = &config.history {
        // Load existing history
        let existing_history = HistoryData::load(history_path, config.verbose)?;

        // Save current data to history first (before checking for changes)
        let new_history = HistoryData {
            last_data: current_report.clone(),
        };
        new_history.save(history_path, config.verbose)?;

        // If --check is specified, compare with history and exit with change code
        if let Some(check_field) = &config.check {
            verbose_println!(&config, "Checking field '{}' for changes", check_field);
            
            if let Some(history) = existing_history {
                let change_magnitude = history.calculate_change(&current_report, check_field)?;
                verbose_println!(&config, "Change magnitude for '{}': {}", check_field, change_magnitude);
                std::process::exit(change_magnitude as i32);
            } else {
                verbose_println!(&config, "No history exists, no change to compare (exit code: 0)");
                std::process::exit(0);
            }
        }
    } else if config.check.is_some() {
        // --check without --history is an error
        anyhow::bail!("--check requires --history to be specified");
    }

    // Print output (unless we exited above for --check)
    print_output(&config, &current_report)?;

    Ok(())
}
