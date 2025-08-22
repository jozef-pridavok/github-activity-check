use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, header};
use serde::Deserialize;
use std::env;

static BASE: &str = "https://api.github.com";

const MIN_COMMITS: usize = 100;
const MIN_CONTRIBUTORS: usize = 3;
const MAX_DAYS_SINCE_LAST_COMMIT: i64 = 60;

const PRS_SCALE: f64 = 10.0;
const ISSUES_SCALE: f64 = 20.0;

#[derive(Debug, Deserialize)]
struct CommitInfo {
    sha: String,
    commit: CommitMeta,
}

#[derive(Debug, Deserialize)]
struct CommitMeta {
    author: AuthorMeta,
    message: String,
}

#[derive(Debug, Deserialize)]
struct AuthorMeta {
    name: String,
    email: String,
    date: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let (owner, repo) = parse_args()?;

    let token = env::var("GITHUB_TOKEN").ok();
    let client = build_client(token.as_deref())?;

    let last_commit = fetch_last_commit(&client, &owner, &repo).await?;

    // commits_count via a more robust function
    let commits_count = fetch_commit_count(&client, &owner, &repo).await?;

    // contributors are fetched via Link header if available
    let contributors_count = fetch_count_via_link(
        &client,
        &format!("/repos/{}/{}/contributors?per_page=1&anon=1", owner, repo),
    )
    .await?;

    // open pull requests (via Link header: per_page=1)
    let open_prs = fetch_count_via_link(
        &client,
        &format!("/repos/{}/{}/pulls?state=open&per_page=1", owner, repo),
    )
    .await?;

    // open issues (use Search API to exclude PRs reliably)
    let open_issues = fetch_open_issues_count(&client, &owner, &repo).await?;

    let alive = is_alive(
        &last_commit.commit.author.date,
        commits_count,
        contributors_count,
        open_prs,
        open_issues,
    );

    println!("Repo: {}/{}", owner, repo);
    println!("-------------------------------------------");
    println!("Commits total            : {}", commits_count);
    println!("Contributors total       : {}", contributors_count);
    println!("Open pull requests       : {}", open_prs);
    println!("Open issues (unresolved) : {}", open_issues);
    println!("Last commit              :");
    println!("  sha                    : {}", last_commit.sha);
    println!(
        "  author                 : {} <{}>",
        last_commit.commit.author.name, last_commit.commit.author.email
    );
    println!("  date (UTC)             : {}", last_commit.commit.author.date);
    println!("  message                : {}", first_line(&last_commit.commit.message));
    println!("-------------------------------------------");
    println!(
        "Project alive           : {}",
        if alive { "ALIVE ✅" } else { "LIKELY DEAD ⚠️" }
    );
    println!(
        "Criteria: last ≤ {} days or (contributors ≥ {} and commits ≥ {})",
        MAX_DAYS_SINCE_LAST_COMMIT, MIN_CONTRIBUTORS, MIN_COMMITS
    );
    Ok(())
}

fn parse_args() -> Result<(String, String)> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("Usage: github-activity-check <owner> <repo>");
        eprintln!("Example: github-activity-check rust-lang rust");
        anyhow::bail!("missing arguments");
    }
    let repo = args.pop().unwrap();
    let owner = args.pop().unwrap();
    Ok((owner, repo))
}

fn build_client(token: Option<&str>) -> Result<Client> {
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
            header::HeaderValue::from_str(&format!("Bearer {}", t))?,
        );
    }
    let client = Client::builder().default_headers(headers).build()?;
    Ok(client)
}

/// Gets the last commit via `commits?per_page=1`
async fn fetch_last_commit(client: &Client, owner: &str, repo: &str) -> Result<CommitInfo> {
    let url = format!("{}/repos/{}/{}/commits?per_page=1", BASE, owner, repo);
    let resp = client.get(url).send().await?.error_for_status()?;
    let mut items: Vec<CommitInfo> = resp.json().await?;
    items.pop().context("Repository has no commits or missing data")
}

/// Robust commit count:
/// 1) Try Link: rel="last" on /commits?per_page=1
/// 2) If it fails/returns 0–1, fallback to Search API `/search/commits?q=repo:owner/repo` (total_count)
async fn fetch_commit_count(client: &Client, owner: &str, repo: &str) -> Result<usize> {
    // primary attempt: Link last
    let via_link = fetch_count_via_link(client, &format!("/repos/{}/{}/commits?per_page=1", owner, repo)).await?;
    if via_link > 1 {
        return Ok(via_link);
    }
    // fallback: Search API (requires Accept: application/vnd.github+json – already added in build_client)
    #[derive(Deserialize)]
    struct SearchCommitsResp {
        total_count: usize,
    }
    let url = format!("{}/search/commits?q=repo:{}/{}", BASE, owner, repo);
    let resp = client.get(url).send().await?.error_for_status()?;
    let body: SearchCommitsResp = resp.json().await?;
    Ok(body.total_count)
}

/// Uses the Search Issues API to count open issues (excludes PRs using `is:issue`).
async fn fetch_open_issues_count(client: &Client, owner: &str, repo: &str) -> Result<usize> {
    #[derive(Deserialize)]
    struct SearchIssuesResp {
        total_count: usize,
    }

    // Search issues: is:issue is:open repo:owner/repo
    let query = format!("q=is:issue+is:open+repo:{}/{}", owner, repo);
    let url = format!("{}/search/issues?{}", BASE, query);
    let resp = client.get(url).send().await?.error_for_status()?;
    let body: SearchIssuesResp = resp.json().await?;
    Ok(body.total_count)
}

/// Returns the total number of objects (commits/contributors) using the Link header.
/// Trick: call the endpoint with `per_page=1` and extract N from `Link: <...page=N>; rel="last"`.
async fn fetch_count_via_link(client: &Client, path_with_query: &str) -> Result<usize> {
    let url = format!("{}{}", BASE, path_with_query);
    let resp = client.get(url).send().await?.error_for_status()?;

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
        if p.ends_with(&format!("rel=\"{}\"", rel)) {
            let start = p.find('<')?;
            let end = p.find('>')?;
            return Some(p[start + 1..end].to_string());
        }
    }
    None
}

fn is_alive(
    last_commit_date: &DateTime<Utc>,
    commits: usize,
    contributors: usize,
    open_prs: usize,
    open_issues: usize,
) -> bool {
    // weights (sum ~ 1.0)
    const W_RECENCY: f64 = 0.40;
    const W_COMMITS: f64 = 0.15;
    const W_CONTRIB: f64 = 0.15;
    const W_PRS: f64 = 0.15;
    const W_ISSUES: f64 = 0.15;

    fn clamp01(x: f64) -> f64 {
        if x < 0.0 {
            0.0
        } else if x > 1.0 {
            1.0
        } else {
            x
        }
    }

    let days_since = (Utc::now() - *last_commit_date).num_days() as f64;

    // recency: decreases linearly to 0 at 2 * MAX_DAYS_SINCE_LAST_COMMIT (smoother transition)
    let recency_scale = (MAX_DAYS_SINCE_LAST_COMMIT as f64) * 2.0;
    let recency_score = clamp01(1.0 - (days_since / recency_scale));

    // other normalized scores
    let commits_score = clamp01(commits as f64 / MIN_COMMITS as f64);
    let contributors_score = clamp01(contributors as f64 / MIN_CONTRIBUTORS as f64);
    let prs_score = clamp01(open_prs as f64 / PRS_SCALE);
    let issues_score = clamp01(open_issues as f64 / ISSUES_SCALE);

    let weighted = recency_score * W_RECENCY
        + commits_score * W_COMMITS
        + contributors_score * W_CONTRIB
        + prs_score * W_PRS
        + issues_score * W_ISSUES;

    // final rule: alive if weighted >= 0.50 OR recency is strong (recent commit)
    weighted >= 0.50 || recency_score >= 0.8
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

// eof
