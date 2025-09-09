use chrono::{DateTime, Utc};
use crate::config::Config;
use crate::types::ReleaseInfo;

// Scoring weights - could be made configurable in the future
pub struct ScoringWeights {
    pub recency: f64,
    pub commits: f64,
    pub contributors: f64,
    pub prs: f64,
    pub issues: f64,
    pub releases: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            recency: 0.35,
            commits: 0.15,
            contributors: 0.15,
            prs: 0.10,
            issues: 0.10,
            releases: 0.15,
        }
    }
}

pub struct ScoringThresholds {
    pub activity_threshold: f64,
    pub recency_threshold: f64,
    pub recency_scale_multiplier: f64,
}

impl Default for ScoringThresholds {
    fn default() -> Self {
        Self {
            activity_threshold: 0.45,
            recency_threshold: 0.8,
            recency_scale_multiplier: 2.0,
        }
    }
}

#[derive(Default)]
pub struct ProjectScorer {
    weights: ScoringWeights,
    thresholds: ScoringThresholds,
}


impl ProjectScorer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_project_alive(
        &self,
        last_commit_date: &DateTime<Utc>,
        commits: usize,
        contributors: usize,
        open_prs: usize,
        open_issues: usize,
        latest_release: Option<&ReleaseInfo>,
        config: &Config,
    ) -> bool {
        let days_since = (Utc::now() - *last_commit_date).num_days() as f64;

        // Recency: decreases linearly to 0 at 2 * max_days (smoother transition)
        let recency_scale = (config.get_max_days() as f64) * self.thresholds.recency_scale_multiplier;
        let recency_score = (1.0 - (days_since / recency_scale)).clamp(0.0, 1.0);

        // Other normalized scores
        let commits_score = (commits as f64 / config.get_min_commits() as f64).clamp(0.0, 1.0);
        let contributors_score = (contributors as f64 / config.get_min_contributors() as f64).clamp(0.0, 1.0);
        let prs_score = (open_prs as f64 / config.get_prs_scale()).clamp(0.0, 1.0);
        let issues_score = (open_issues as f64 / config.get_issues_scale()).clamp(0.0, 1.0);
        
        // Release scoring: recent release = high score, old release = lower score
        let release_score = if let Some(release) = latest_release {
            if let Some(published_at) = release.published_at {
                let release_days_ago = (Utc::now() - published_at).num_days() as f64;
                let max_release_days = config.get_max_release_days() as f64;
                
                // Score decreases linearly with age, prerelease versions get penalty
                let base_score = (1.0 - (release_days_ago / max_release_days)).clamp(0.0, 1.0);
                if release.prerelease {
                    base_score * 0.7  // 30% penalty for prereleases
                } else {
                    base_score
                }
            } else {
                0.5  // Release exists but no date = medium score
            }
        } else {
            0.0  // No releases = no score
        };

        let weighted_score = recency_score * self.weights.recency
            + commits_score * self.weights.commits
            + contributors_score * self.weights.contributors
            + prs_score * self.weights.prs
            + issues_score * self.weights.issues
            + release_score * self.weights.releases;

        // Final rule: alive if weighted score >= threshold OR recency is strong (recent commit)
        weighted_score >= self.thresholds.activity_threshold || recency_score >= self.thresholds.recency_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_config() -> Config {
        Config {
            owner: Some("test".to_string()),
            repo: Some("repo".to_string()),
            min_commits: Some(100),
            min_contributors: Some(3),
            max_days: Some(60),
            prs_scale: Some(10.0),
            issues_scale: Some(20.0),
            max_release_days: Some(365),
            ..Default::default()
        }
    }

    #[test]
    fn test_recent_commit_is_alive() {
        let scorer = ProjectScorer::new();
        let config = create_test_config();
        let recent_date = Utc::now() - chrono::Duration::days(1);
        
        let result = scorer.is_project_alive(&recent_date, 50, 1, 0, 0, None, &config);
        assert!(result, "Recent commit should make project alive");
    }

    #[test]
    fn test_old_but_established_project_is_alive() {
        let scorer = ProjectScorer::new();
        let config = create_test_config();
        let old_date = Utc::now() - chrono::Duration::days(100);
        
        let result = scorer.is_project_alive(&old_date, 1000, 10, 5, 10, None, &config);
        assert!(result, "Established project should be alive even with old commits");
    }

    #[test]
    fn test_old_and_small_project_is_dead() {
        let scorer = ProjectScorer::new();
        let config = create_test_config();
        let old_date = Utc::now() - chrono::Duration::days(200);
        
        let result = scorer.is_project_alive(&old_date, 10, 1, 0, 0, None, &config);
        assert!(!result, "Old and small project should be dead");
    }

    #[test]
    fn test_edge_case_exact_thresholds() {
        let scorer = ProjectScorer::new();
        let config = create_test_config();
        let threshold_date = Utc::now() - chrono::Duration::days(60);
        
        // Exactly at thresholds
        let result = scorer.is_project_alive(&threshold_date, 100, 3, 10, 20, None, &config);
        assert!(result, "Project at exact thresholds should be alive");
    }
}