//! GitHub repository discovery for training validation candidates.
//! 
//! Discovers public repositories with active issues and PRs, recent updates,
//! and sufficient community engagement (50+ stars) suitable for training validation.

use serde::{Deserialize, Serialize};
use reqwest::Client;
use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("GitHub API error: {0}")]
    ApiError(String),
    
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Rate limit exceeded")]
    RateLimited,
    
    #[error("Repository not found: {0}")]
    RepoNotFound(String),
}

/// Represents rate limit status from GitHub API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: chrono::DateTime<chrono::Utc>,
}

/// Represents a repository candidate for training validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCandidate {
    pub url: String,
    pub name: String,
    pub owner: String,
    pub stars: u32,
    pub language: Option<String>,
    pub issue_count: u32,
    pub pr_count: u32,
    pub last_issue_id: Option<String>,
    pub last_pr_id: Option<String>,
    pub updated_at: String,
}

/// Represents an issue with its linked PR data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueWithPR {
    pub issue_title: String,
    pub issue_body: String,
    pub issue_number: u32,
    pub pr_files: Vec<String>,
    pub pr_title: Option<String>,
    pub pr_number: Option<u32>,
}

/// GitHub GraphQL response for repository discovery
#[derive(Debug, Deserialize)]
struct GitHubGraphQLResponse {
    data: Option<serde_json::Value>,
    errors: Option<Vec<serde_json::Value>>,
}

const GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Discovers training candidate repositories from GitHub
/// 
/// Returns repositories with:
/// - Both issues and PRs (activity indicator)
/// - Recent updates (> 30 days ago)
/// - Minimum 50 stars (community engagement)
pub async fn discover_training_repos(limit: usize) -> Result<Vec<RepoCandidate>, DiscoveryError> {
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    // Calculate 30 days ago for recent activity filter
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
    let date_filter = thirty_days_ago.format("%Y-%m-%d").to_string();

    // GraphQL query for repository discovery
    let query = serde_json::json!({
        "query": format!(r#"
            query {{
                search(query: "stars:>50 issues:>0 pr:>0 is:public sort:updated-desc updated:>{}", 
                       type: REPOSITORY, first: 100) {{
                    nodes {{
                        ... on Repository {{
                            name
                            url
                            owner {{
                                login
                            }}
                            stargazers {{
                                totalCount
                            }}
                            primaryLanguage {{
                                name
                            }}
                            issues(states: OPEN) {{
                                totalCount
                                nodes(last: 1) {{
                                    id
                                }}
                            }}
                            pullRequests(states: OPEN) {{
                                totalCount
                                nodes(last: 1) {{
                                    id
                                }}
                            }}
                            updatedAt
                        }}
                    }}
                }}
                rateLimit {{
                    limit
                    remaining
                    resetAt
                }}
            }}
        "#, date_filter)
    });

    let response = client
        .post(GITHUB_GRAPHQL_URL)
        .json(&query)
        .send()
        .await?;

    let status = response.status();
    let _rate_limit = parse_rate_limit_headers(&response);

    if status == reqwest::StatusCode::FORBIDDEN {
        return Err(DiscoveryError::RateLimited);
    }

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(DiscoveryError::ApiError(format!(
            "HTTP {}: {}",
            status, body
        )));
    }

    let gh_response: GitHubGraphQLResponse = response.json().await?;

    if let Some(errors) = gh_response.errors {
        let error_msg = errors
            .iter()
            .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(DiscoveryError::ApiError(error_msg));
    }

    let data = gh_response.data.ok_or_else(|| {
        DiscoveryError::ApiError("No data in response".to_string())
    })?;

    // Parse search results
    let mut candidates = Vec::new();
    
    if let Some(search_nodes) = data
        .get("search")
        .and_then(|s| s.get("nodes"))
        .and_then(|n| n.as_array())
    {
        for repo_json in search_nodes.iter().take(limit) {
            if let Ok(candidate) = parse_repo_candidate(repo_json) {
                // Filter: must have both issues and PRs
                if candidate.issue_count > 0 && candidate.pr_count > 0 {
                    candidates.push(candidate);
                }
            }
        }
    }

    if let Some(rate_limit_data) = data.get("rateLimit") {
        log_rate_limit(rate_limit_data);
    }

    Ok(candidates)
}

/// Fetches an issue and its linked pull request
pub async fn fetch_issue_and_linked_pr(
    repo_owner: &str,
    repo_name: &str,
    issue_number: u32,
) -> Result<IssueWithPR, DiscoveryError> {
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;

    // First, fetch the issue
    let issue_query = serde_json::json!({
        "query": format!(
            r#"
            query {{
                repository(owner: "{}", name: "{}") {{
                    issue(number: {}) {{
                        title
                        body
                        number
                        timelineItems(first: 100, itemTypes: [CONNECTED_EVENT, DISCONNECTED_EVENT]) {{
                            nodes {{
                                ... on ConnectedEvent {{
                                    subject {{
                                        ... on PullRequest {{
                                            number
                                            title
                                            files(first: 20) {{
                                                nodes {{
                                                    path
                                                }}
                                            }}
                                        }}
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}
            }}
            "#,
            repo_owner, repo_name, issue_number
        )
    });

    let response = client
        .post(GITHUB_GRAPHQL_URL)
        .json(&issue_query)
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(DiscoveryError::RateLimited);
    }

    let gh_response: GitHubGraphQLResponse = response.json().await?;

    if let Some(errors) = gh_response.errors {
        let error_msg = errors
            .iter()
            .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(DiscoveryError::ApiError(error_msg));
    }

    let data = gh_response.data.ok_or_else(|| {
        DiscoveryError::ApiError("No data in response".to_string())
    })?;

    parse_issue_with_pr(&data, repo_owner, repo_name, issue_number)
}

// Helper functions

fn parse_repo_candidate(repo_json: &serde_json::Value) -> Result<RepoCandidate, DiscoveryError> {
    let name = repo_json
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| DiscoveryError::ApiError("Missing repo name".to_string()))?
        .to_string();

    let url = repo_json
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| DiscoveryError::ApiError("Missing repo URL".to_string()))?
        .to_string();

    let owner = repo_json
        .get("owner")
        .and_then(|o| o.get("login"))
        .and_then(|l| l.as_str())
        .ok_or_else(|| DiscoveryError::ApiError("Missing owner".to_string()))?
        .to_string();

    let stars = repo_json
        .get("stargazers")
        .and_then(|s| s.get("totalCount"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let language = repo_json
        .get("primaryLanguage")
        .and_then(|l| l.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let issue_count = repo_json
        .get("issues")
        .and_then(|i| i.get("totalCount"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let pr_count = repo_json
        .get("pullRequests")
        .and_then(|p| p.get("totalCount"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    let last_issue_id = repo_json
        .get("issues")
        .and_then(|i| i.get("nodes"))
        .and_then(|n| n.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string());

    let last_pr_id = repo_json
        .get("pullRequests")
        .and_then(|p| p.get("nodes"))
        .and_then(|n| n.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string());

    let updated_at = repo_json
        .get("updatedAt")
        .and_then(|u| u.as_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(RepoCandidate {
        url,
        name,
        owner,
        stars,
        language,
        issue_count,
        pr_count,
        last_issue_id,
        last_pr_id,
        updated_at,
    })
}

fn parse_issue_with_pr(
    data: &serde_json::Value,
    repo_owner: &str,
    repo_name: &str,
    issue_number: u32,
) -> Result<IssueWithPR, DiscoveryError> {
    let repo = data
        .get("repository")
        .ok_or_else(|| DiscoveryError::RepoNotFound(format!("{}/{}", repo_owner, repo_name)))?;

    let issue = repo
        .get("issue")
        .ok_or_else(|| {
            DiscoveryError::ApiError(format!("Issue #{} not found", issue_number))
        })?;

    let title = issue
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let body = issue
        .get("body")
        .and_then(|b| b.as_str())
        .unwrap_or("")
        .to_string();

    let number = issue
        .get("number")
        .and_then(|n| n.as_u64())
        .unwrap_or(issue_number as u64) as u32;

    // Extract linked PR from timeline events
    let mut pr_files = Vec::new();
    let mut pr_title = None;
    let mut pr_number = None;

    if let Some(timeline) = issue.get("timelineItems").and_then(|t| t.get("nodes")) {
        if let Some(events) = timeline.as_array() {
            for event in events {
                if let Some(subject) = event.get("subject") {
                    if let Some(pr) = subject.get("number") {
                        pr_number = pr.as_u64().map(|n| n as u32);
                    }
                    if let Some(pr_t) = subject.get("title") {
                        pr_title = pr_t.as_str().map(|s| s.to_string());
                    }
                    if let Some(files) = subject
                        .get("files")
                        .and_then(|f| f.get("nodes"))
                        .and_then(|n| n.as_array())
                    {
                        for file in files {
                            if let Some(path) = file.get("path").and_then(|p| p.as_str()) {
                                pr_files.push(path.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(IssueWithPR {
        issue_title: title,
        issue_body: body,
        issue_number: number,
        pr_files,
        pr_title,
        pr_number,
    })
}

fn parse_rate_limit_headers(response: &reqwest::Response) -> Option<RateLimitStatus> {
    let limit = response
        .headers()
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())?;

    let remaining = response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())?;

    let reset = response
        .headers()
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .map(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0))
        .flatten()?;

    Some(RateLimitStatus {
        limit,
        remaining,
        reset_at: reset,
    })
}

fn log_rate_limit(rate_limit: &serde_json::Value) {
    if let (Some(limit), Some(remaining), Some(reset)) = (
        rate_limit.get("limit").and_then(|l| l.as_u64()),
        rate_limit.get("remaining").and_then(|r| r.as_u64()),
        rate_limit.get("resetAt").and_then(|r| r.as_str()),
    ) {
        tracing::info!(
            "GitHub API rate limit: {}/{} remaining, resets at {}",
            remaining,
            limit,
            reset
        );
    }
}
