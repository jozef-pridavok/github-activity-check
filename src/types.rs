use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitInfo {
    pub sha: String,
    pub commit: CommitMeta,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitMeta {
    pub author: AuthorMeta,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorMeta {
    pub name: String,
    pub email: String,
    pub date: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct SearchCommitsResp {
    pub total_count: usize,
}

#[derive(Deserialize)]
pub struct SearchIssuesResp {
    pub total_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub prerelease: bool,
    pub draft: bool,
}