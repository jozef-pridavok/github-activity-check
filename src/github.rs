use anyhow::{Context, Result};
use reqwest::{Client, header};

use crate::types::{CommitInfo, ReleaseInfo, SearchCommitsResp, SearchIssuesResp};

static BASE: &str = "https://api.github.com";

pub struct GitHubClient {
    client: Client,
}

impl GitHubClient {
    pub fn new(token: Option<&str>) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("github-activity-check/0.1"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        if let Some(t) = token {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {t}"))?,
            );
        }
        let client = Client::builder().default_headers(headers).build()?;
        Ok(GitHubClient { client })
    }

    pub async fn get_last_commit(&self, owner: &str, repo: &str) -> Result<CommitInfo> {
        let url = format!("{BASE}/repos/{owner}/{repo}/commits?per_page=1");
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to fetch commits from {url}"))?
            .error_for_status()
            .with_context(|| format!("GitHub API error for repository {owner}/{repo}"))?;
        let mut items: Vec<CommitInfo> = resp.json().await
            .context("Failed to parse commit response as JSON")?;
        items.pop().with_context(|| format!("Repository {owner}/{repo} has no commits"))
    }

    pub async fn get_commit_count(&self, owner: &str, repo: &str) -> Result<usize> {
        // Primary attempt: Link last
        let via_link = self.fetch_count_via_link(&format!("/repos/{owner}/{repo}/commits?per_page=1")).await?;
        if via_link > 1 {
            return Ok(via_link);
        }
        
        // Fallback: Search API
        let url = format!("{BASE}/search/commits?q=repo:{owner}/{repo}");
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to search commits from {url}"))?
            .error_for_status()
            .with_context(|| format!("Search API error for repository {owner}/{repo}"))?;
        let body: SearchCommitsResp = resp.json().await
            .context("Failed to parse search commits response")?;
        Ok(body.total_count)
    }

    pub async fn get_contributors_count(&self, owner: &str, repo: &str) -> Result<usize> {
        self.fetch_count_via_link(&format!("/repos/{owner}/{repo}/contributors?per_page=1&anon=1")).await
    }

    pub async fn get_open_prs_count(&self, owner: &str, repo: &str) -> Result<usize> {
        self.fetch_count_via_link(&format!("/repos/{owner}/{repo}/pulls?state=open&per_page=1")).await
    }

    pub async fn get_open_issues_count(&self, owner: &str, repo: &str) -> Result<usize> {
        let query = format!("q=is:issue+is:open+repo:{owner}/{repo}");
        let url = format!("{BASE}/search/issues?{query}");
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to search issues from {url}"))?
            .error_for_status()
            .with_context(|| format!("Issues search API error for repository {owner}/{repo}"))?;
        let body: SearchIssuesResp = resp.json().await
            .context("Failed to parse search issues response")?;
        Ok(body.total_count)
    }

    pub async fn get_latest_release(&self, owner: &str, repo: &str) -> Result<Option<ReleaseInfo>> {
        let url = format!("{BASE}/repos/{owner}/{repo}/releases/latest");
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to fetch latest release from {url}"))?;
        
        // GitHub returns 404 if no releases exist
        if resp.status() == 404 {
            return Ok(None);
        }
        
        let resp = resp.error_for_status()
            .with_context(|| format!("Latest release API error for repository {owner}/{repo}"))?;
        
        let release: ReleaseInfo = resp.json().await
            .context("Failed to parse latest release response")?;
        
        Ok(Some(release))
    }

    async fn fetch_count_via_link(&self, path_with_query: &str) -> Result<usize> {
        let url = format!("{BASE}{path_with_query}");
        let resp = self.client.get(&url).send().await
            .with_context(|| format!("Failed to fetch data from {url}"))?
            .error_for_status()
            .with_context(|| format!("GitHub API error for endpoint: {path_with_query}"))?;

        if let Some(link) = resp.headers().get(header::LINK) {
            let link_str = link.to_str().unwrap_or_default();
            if let Some(last_page) = parse_last_page(link_str) {
                return Ok(last_page);
            }
            // if not `last`, there may be at least `next` → we know results are >= 2
            if parse_rel_url(link_str, "next").is_some() {
                return Ok(2); // at least 2 (conservative estimate)
            }
        }

        // Without Link: count from body (0 or 1)
        let text = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&text).context("Invalid JSON response")?;
        if let Some(arr) = v.as_array() {
            return Ok(arr.len());
        }
        Ok(0)
    }
}

fn parse_last_page(link_header: &str) -> Option<usize> {
    // Look for the segment with rel="last", extract page=
    for part in link_header.split(',') {
        let part = part.trim();
        if part.contains("rel=\"last\"") {
            let start = part.find('<')?;
            let end = part.find('>')?;
            let url = &part[start + 1..end];
            // Look for the query "page="
            // Split on '?', then by '&'
            let query = url.split('?').nth(1)?;
            for kv in query.split('&') {
                let mut it = kv.splitn(2, '=');
                let k = it.next()?;
                let v = it.next().unwrap_or("");
                if k == "page" {
                    if let Ok(n) = v.parse::<usize>() {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

// Helper: returns the URL for the given rel (e.g., "next"), if it exists
fn parse_rel_url(link_header: &str, rel: &str) -> Option<String> {
    for part in link_header.split(',') {
        let p = part.trim();
        if p.ends_with(&format!("rel=\"{rel}\"")) {
            let start = p.find('<')?;
            let end = p.find('>')?;
            return Some(p[start + 1..end].to_string());
        }
    }
    None
}