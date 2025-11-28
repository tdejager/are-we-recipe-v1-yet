use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

const GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";
const BATCH_SIZE: usize = 50;

/// GitHub GraphQL client for querying repository information
pub struct GitHubClient {
    client: reqwest::Client,
    token: String,
}

/// Commit author information from GraphQL response
#[derive(Debug, Clone)]
pub struct CommitAuthor {
    /// GitHub username (None for external/bot commits without linked account)
    pub login: Option<String>,
    pub name: String,
    pub email: String,
}

/// Result of querying recipe.yaml history for a feedstock
#[derive(Debug)]
pub struct RecipeHistoryResult {
    pub feedstock: String,
    pub first_recipe_commit: Option<FirstRecipeCommit>,
    pub error: Option<String>,
}

/// Information about the first commit that added recipe.yaml
#[derive(Debug, Clone)]
pub struct FirstRecipeCommit {
    pub sha: String,
    pub author: CommitAuthor,
    pub date: String,
    pub message: String,
}

/// Rate limit information from GitHub API
#[derive(Debug)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: String,
}

/// Information about a Pull Request
#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    pub number: u32,
    pub author: String,
}

/// A commit within a Pull Request
#[derive(Debug, Clone)]
pub struct PrCommit {
    pub sha: String,
    pub author: String,
    pub files_changed: Vec<String>,
}

#[derive(Deserialize)]
struct GraphQLResponse {
    data: Option<serde_json::Value>,
    errors: Option<Vec<serde_json::Value>>,
}

impl GitHubClient {
    /// Create a new GitHub client with token resolution:
    /// 1. Try `gh auth token` command (for local dev)
    /// 2. Fall back to `GITHUB_TOKEN` env var
    /// 3. Fall back to `GH_TOKEN` env var
    pub fn new() -> Result<Self> {
        let token = Self::resolve_token()?;

        let client = reqwest::Client::builder()
            .user_agent("are-we-recipe-v1-yet/1.0")
            .build()?;

        Ok(Self { client, token })
    }

    fn resolve_token() -> Result<String> {
        // Try gh CLI first (for local development)
        // Note: Clear GITHUB_TOKEN/GH_TOKEN env vars when calling gh, otherwise gh will just
        // echo those tokens back instead of returning its own authenticated token
        if let Ok(output) = Command::new("gh")
            .args(["auth", "token"])
            .env_remove("GITHUB_TOKEN")
            .env_remove("GH_TOKEN")
            .output()
        {
            if output.status.success() {
                let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !token.is_empty() {
                    return Ok(token);
                }
            }
        }

        // Fall back to environment variables
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                return Ok(token);
            }
        }

        if let Ok(token) = std::env::var("GH_TOKEN") {
            if !token.is_empty() {
                return Ok(token);
            }
        }

        Err(anyhow::anyhow!(
            "No GitHub token found. Install gh CLI and run 'gh auth login', \
             or set GITHUB_TOKEN/GH_TOKEN environment variable."
        ))
    }

    /// Check remaining rate limit
    pub async fn check_rate_limit(&self) -> Result<RateLimitInfo> {
        let query = r#"query { rateLimit { limit remaining resetAt } }"#;

        let response = self.execute_query(query).await?;
        let rate_limit = response
            .get("rateLimit")
            .context("No rateLimit in response")?;

        Ok(RateLimitInfo {
            limit: rate_limit["limit"].as_u64().unwrap_or(0) as u32,
            remaining: rate_limit["remaining"].as_u64().unwrap_or(0) as u32,
            reset_at: rate_limit["resetAt"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Batch query multiple feedstocks for their first recipe.yaml commit
    pub async fn batch_query_recipe_history(
        &self,
        feedstocks: &[String],
    ) -> Result<Vec<RecipeHistoryResult>> {
        if feedstocks.is_empty() {
            return Ok(vec![]);
        }

        // Process in chunks of BATCH_SIZE
        let mut all_results = Vec::new();
        let mut all_pagination_needed = Vec::new();
        let total_chunks = (feedstocks.len() + BATCH_SIZE - 1) / BATCH_SIZE;

        for (i, chunk) in feedstocks.chunks(BATCH_SIZE).enumerate() {
            eprint!("\r📦 Batch {}/{} ({} feedstocks)...", i + 1, total_chunks, all_results.len());
            let query = build_batch_query(chunk);
            let response = self.execute_query(&query).await?;
            let (results, pagination_needed) = parse_batch_response(chunk, &response)?;
            all_results.extend(results);
            all_pagination_needed.extend(pagination_needed);
        }
        eprintln!("\r📦 Processed {} feedstocks in {} batches", all_results.len(), total_chunks);

        // Handle feedstocks that need pagination (>100 commits to recipe.yaml)
        if !all_pagination_needed.is_empty() {
            println!(
                "📄 {} feedstocks need pagination for full commit history",
                all_pagination_needed.len()
            );

            for pag in &all_pagination_needed {
                eprintln!("  Paginating: {} (path: {})...", pag.feedstock, pag.path);
                if let Some(commit) = self.paginate_to_oldest_commit(pag).await? {
                    // Update the result for this feedstock
                    if let Some(result) = all_results
                        .iter_mut()
                        .find(|r| r.feedstock == pag.feedstock)
                    {
                        result.first_recipe_commit = Some(commit);
                    }
                }
            }
        }

        Ok(all_results)
    }

    /// Paginate through commit history to find the oldest commit
    async fn paginate_to_oldest_commit(
        &self,
        pag: &PaginationNeeded,
    ) -> Result<Option<FirstRecipeCommit>> {
        let mut cursor = pag.cursor.clone();
        let mut oldest_commit = pag.oldest_commit_so_far.clone();
        let mut page_count = 0;
        const MAX_PAGES: usize = 50; // Safety limit: 50 pages * 100 = 5000 commits max

        loop {
            page_count += 1;
            if page_count > MAX_PAGES {
                eprintln!("    Warning: Hit max page limit ({}) for {}", MAX_PAGES, pag.feedstock);
                break;
            }
            let query = format!(
                r#"query {{
                    repository(owner: "conda-forge", name: "{feedstock}") {{
                        defaultBranchRef {{
                            target {{
                                ... on Commit {{
                                    history(first: 100, path: "{path}", after: "{cursor}") {{
                                        pageInfo {{
                                            hasNextPage
                                            endCursor
                                        }}
                                        nodes {{
                                            oid
                                            message
                                            committedDate
                                            author {{
                                                user {{ login }}
                                                name
                                                email
                                            }}
                                        }}
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}"#,
                feedstock = pag.feedstock,
                path = pag.path,
                cursor = cursor
            );

            let response = self.execute_query(&query).await?;

            let history = response
                .get("repository")
                .and_then(|r| r.get("defaultBranchRef"))
                .and_then(|b| b.get("target"))
                .and_then(|t| t.get("history"));

            let Some(history) = history else {
                break;
            };

            let nodes = history.get("nodes").and_then(|n| n.as_array());
            let Some(nodes) = nodes else {
                break;
            };

            // Update oldest commit if we have nodes
            if let Some(commit) = nodes.last() {
                if let Some(author) = commit.get("author") {
                    oldest_commit = FirstRecipeCommit {
                        sha: commit
                            .get("oid")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        message: commit
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        date: commit
                            .get("committedDate")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        author: CommitAuthor {
                            login: author
                                .get("user")
                                .and_then(|u| u.get("login"))
                                .and_then(|l| l.as_str())
                                .map(String::from),
                            name: author
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            email: author
                                .get("email")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        },
                    };
                }
            }

            // Check if there are more pages
            let page_info = history.get("pageInfo");
            let has_next = page_info
                .and_then(|p| p.get("hasNextPage"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !has_next {
                break;
            }

            cursor = page_info
                .and_then(|p| p.get("endCursor"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if cursor.is_empty() {
                break;
            }
        }

        Ok(Some(oldest_commit))
    }

    /// Get the PR that introduced a specific commit (if any)
    pub async fn get_pr_for_commit(
        &self,
        feedstock: &str,
        commit_sha: &str,
    ) -> Result<Option<PullRequestInfo>> {
        // Use REST API: GET /repos/{owner}/{repo}/commits/{commit_sha}/pulls
        let url = format!(
            "https://api.github.com/repos/conda-forge/{}/commits/{}/pulls",
            feedstock, commit_sha
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "are-we-recipe-v1-yet/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let prs: Vec<serde_json::Value> = response.json().await?;

        // Return the first (most recent) PR that contains this commit
        if let Some(pr) = prs.first() {
            let number = pr["number"].as_u64().unwrap_or(0) as u32;
            let author = pr["user"]["login"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            return Ok(Some(PullRequestInfo { number, author }));
        }

        Ok(None)
    }

    /// Get commits in a PR with file change info
    pub async fn get_pr_commits(
        &self,
        feedstock: &str,
        pr_number: u32,
    ) -> Result<Vec<PrCommit>> {
        let url = format!(
            "https://api.github.com/repos/conda-forge/{}/pulls/{}/commits",
            feedstock, pr_number
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "are-we-recipe-v1-yet/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let commits: Vec<serde_json::Value> = response.json().await?;
        let mut result = Vec::new();

        for commit in commits {
            let sha = commit["sha"].as_str().unwrap_or("").to_string();
            let author = commit["author"]["login"]
                .as_str()
                .or_else(|| commit["commit"]["author"]["name"].as_str())
                .unwrap_or("unknown")
                .to_string();

            // We need to fetch each commit individually to get file changes
            // This is expensive, so we'll do it lazily only when needed
            result.push(PrCommit {
                sha,
                author,
                files_changed: vec![], // Will be populated on demand
            });
        }

        Ok(result)
    }

    /// Check if a specific commit contains recipe.yaml in its changed files
    pub async fn commit_has_recipe_yaml(&self, feedstock: &str, commit_sha: &str) -> Result<bool> {
        let url = format!(
            "https://api.github.com/repos/conda-forge/{}/commits/{}",
            feedstock, commit_sha
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "are-we-recipe-v1-yet/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let commit: serde_json::Value = response.json().await?;

        if let Some(files) = commit["files"].as_array() {
            for file in files {
                if let Some(filename) = file["filename"].as_str() {
                    if filename == "recipe.yaml" || filename == "recipe/recipe.yaml" {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check if recipe.yaml exists in the very first commit of the repo
    pub async fn has_recipe_yaml_in_first_commit(&self, feedstock: &str) -> Result<bool> {
        // Find the first commit by paginating through REST API until we find one with no parents
        let first_commit_sha = self.find_first_commit_sha(feedstock).await?;

        let Some(sha) = first_commit_sha else {
            return Ok(false);
        };

        // Check if recipe.yaml exists in this commit
        self.check_recipe_yaml_in_recipe_dir(feedstock, &sha).await
    }

    /// Find the SHA of the very first commit in the repository using GraphQL
    async fn find_first_commit_sha(&self, feedstock: &str) -> Result<Option<String>> {
        let mut cursor: Option<String> = None;

        loop {
            let after_clause = cursor
                .as_ref()
                .map(|c| format!(r#", after: "{}""#, c))
                .unwrap_or_default();

            let query = format!(
                r#"query {{
                    repository(owner: "conda-forge", name: "{}") {{
                        defaultBranchRef {{
                            target {{
                                ... on Commit {{
                                    history(first: 100{}) {{
                                        pageInfo {{
                                            hasNextPage
                                            endCursor
                                        }}
                                        nodes {{
                                            oid
                                            parents {{
                                                totalCount
                                            }}
                                        }}
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}"#,
                feedstock, after_clause
            );

            let response = self.execute_query(&query).await?;

            let history = response
                .get("repository")
                .and_then(|r| r.get("defaultBranchRef"))
                .and_then(|b| b.get("target"))
                .and_then(|t| t.get("history"));

            let Some(history) = history else {
                return Ok(None);
            };

            let nodes = history.get("nodes").and_then(|n| n.as_array());
            let Some(nodes) = nodes else {
                return Ok(None);
            };

            // Find commit with no parents (the first commit)
            for node in nodes {
                let parent_count = node
                    .get("parents")
                    .and_then(|p| p.get("totalCount"))
                    .and_then(|c| c.as_u64())
                    .unwrap_or(1);

                if parent_count == 0 {
                    return Ok(node.get("oid").and_then(|o| o.as_str()).map(String::from));
                }
            }

            // Check if there are more pages
            let has_next = history
                .get("pageInfo")
                .and_then(|p| p.get("hasNextPage"))
                .and_then(|h| h.as_bool())
                .unwrap_or(false);

            if !has_next {
                return Ok(None);
            }

            cursor = history
                .get("pageInfo")
                .and_then(|p| p.get("endCursor"))
                .and_then(|c| c.as_str())
                .map(String::from);
        }
    }

    /// Helper to check if recipe/recipe.yaml exists in a specific commit
    async fn check_recipe_yaml_in_recipe_dir(
        &self,
        feedstock: &str,
        commit_sha: &str,
    ) -> Result<bool> {
        let url = format!(
            "https://raw.githubusercontent.com/conda-forge/{}/{}/recipe/recipe.yaml",
            feedstock, commit_sha
        );

        let response = self.client.head(&url).send().await?;
        Ok(response.status().is_success())
    }

    async fn execute_query(&self, query: &str) -> Result<serde_json::Value> {
        self.execute_query_with_retries(query, 3).await
    }

    async fn execute_query_with_retries(
        &self,
        query: &str,
        max_retries: u32,
    ) -> Result<serde_json::Value> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                let delay = std::time::Duration::from_secs(1 << (attempt - 1));
                tokio::time::sleep(delay).await;
            }

            let response = self
                .client
                .post(GITHUB_GRAPHQL_URL)
                .header("Authorization", format!("Bearer {}", self.token))
                .json(&serde_json::json!({ "query": query }))
                .send()
                .await?;

            let status = response.status();

            // Retry on 5xx errors
            if status.is_server_error() {
                let body = response.text().await.unwrap_or_default();
                last_error = Some(anyhow::anyhow!(
                    "GitHub API error: {}. Response: {}",
                    status,
                    body.chars().take(200).collect::<String>()
                ));
                continue;
            }

            if status == 401 {
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "GitHub API authentication failed (401). Response: {}. \
                     Token prefix: {}...",
                    body.chars().take(200).collect::<String>(),
                    self.token.chars().take(10).collect::<String>()
                ));
            }
            if status == 403 {
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "GitHub API forbidden (403). Response: {}",
                    body.chars().take(200).collect::<String>()
                ));
            }
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "GitHub API error: {}. Response: {}",
                    status,
                    body.chars().take(200).collect::<String>()
                ));
            }

            let result: GraphQLResponse = response.json().await?;

            if let Some(errors) = result.errors {
                // Log errors but continue - some repos may not exist
                for error in &errors {
                    if let Some(msg) = error.get("message").and_then(|m| m.as_str()) {
                        // Only warn for non-NOT_FOUND errors
                        if !msg.contains("Could not resolve") {
                            eprintln!("GraphQL warning: {}", msg);
                        }
                    }
                }
            }

            return result.data.context("No data in GraphQL response");
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }
}

/// Build a batched GraphQL query for multiple feedstocks
fn build_batch_query(feedstocks: &[String]) -> String {
    let mut query = String::from("query {\n");

    for (i, feedstock) in feedstocks.iter().enumerate() {
        // Query both possible recipe.yaml locations
        query.push_str(&format!(
            r#"
            repo{i}: repository(owner: "conda-forge", name: "{feedstock}") {{
                name
                defaultBranchRef {{
                    target {{
                        ... on Commit {{
                            historyMain: history(first: 100, path: "recipe.yaml") {{
                                totalCount
                                pageInfo {{
                                    hasNextPage
                                    endCursor
                                }}
                                nodes {{
                                    oid
                                    message
                                    committedDate
                                    author {{
                                        user {{ login }}
                                        name
                                        email
                                    }}
                                }}
                            }}
                            historyAlt: history(first: 100, path: "recipe/recipe.yaml") {{
                                totalCount
                                pageInfo {{
                                    hasNextPage
                                    endCursor
                                }}
                                nodes {{
                                    oid
                                    message
                                    committedDate
                                    author {{
                                        user {{ login }}
                                        name
                                        email
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}
            }}
            "#,
            i = i,
            feedstock = feedstock
        ));
    }

    query.push_str("\n}");
    query
}

/// Info about a feedstock that needs pagination to get all commits
#[derive(Debug)]
struct PaginationNeeded {
    feedstock: String,
    path: &'static str,
    cursor: String,
    oldest_commit_so_far: FirstRecipeCommit,
}

/// Parse the batched response and extract commit information
fn parse_batch_response(
    feedstocks: &[String],
    response: &serde_json::Value,
) -> Result<(Vec<RecipeHistoryResult>, Vec<PaginationNeeded>)> {
    let mut results = Vec::new();
    let mut needs_pagination = Vec::new();

    for (i, feedstock) in feedstocks.iter().enumerate() {
        let repo_key = format!("repo{}", i);
        let repo_data = response.get(&repo_key);

        let result = match repo_data {
            Some(repo) if !repo.is_null() => {
                // Try main path first, then alt path
                let (commit, pagination) = extract_first_commit_with_pagination(repo, "historyMain", "recipe.yaml", feedstock)
                    .or_else(|| extract_first_commit_with_pagination(repo, "historyAlt", "recipe/recipe.yaml", feedstock))
                    .unwrap_or((None, None));

                if let Some(pag) = pagination {
                    needs_pagination.push(pag);
                }

                RecipeHistoryResult {
                    feedstock: feedstock.clone(),
                    first_recipe_commit: commit,
                    error: None,
                }
            }
            _ => RecipeHistoryResult {
                feedstock: feedstock.clone(),
                first_recipe_commit: None,
                error: Some("Repository not found or no recipe.yaml".to_string()),
            },
        };

        results.push(result);
    }

    Ok((results, needs_pagination))
}

/// Extract the oldest commit from the history, returning pagination info if more pages exist
fn extract_first_commit_with_pagination(
    repo: &serde_json::Value,
    history_key: &str,
    path: &'static str,
    feedstock: &str,
) -> Option<(Option<FirstRecipeCommit>, Option<PaginationNeeded>)> {
    let history = repo
        .get("defaultBranchRef")?
        .get("target")?
        .get(history_key)?;

    let nodes = history.get("nodes")?.as_array()?;
    if nodes.is_empty() {
        return None;
    }

    let page_info = history.get("pageInfo")?;
    let has_next_page = page_info.get("hasNextPage")?.as_bool().unwrap_or(false);

    // Get the last (oldest) commit from this page - GitHub returns commits in reverse chronological order
    let commit = nodes.last()?;
    let author = commit.get("author")?;

    let oldest_commit = FirstRecipeCommit {
        sha: commit.get("oid")?.as_str()?.to_string(),
        message: commit.get("message")?.as_str()?.to_string(),
        date: commit.get("committedDate")?.as_str()?.to_string(),
        author: CommitAuthor {
            login: author
                .get("user")
                .and_then(|u| u.get("login"))
                .and_then(|l| l.as_str())
                .map(String::from),
            name: author.get("name")?.as_str()?.to_string(),
            email: author.get("email")?.as_str()?.to_string(),
        },
    };

    let pagination = if has_next_page {
        let cursor = page_info.get("endCursor")?.as_str()?.to_string();
        Some(PaginationNeeded {
            feedstock: feedstock.to_string(),
            path,
            cursor,
            oldest_commit_so_far: oldest_commit.clone(),
        })
    } else {
        None
    };

    // If there's more pages, we return None for commit (will be filled by pagination)
    // Otherwise return the oldest commit we found
    if has_next_page {
        Some((None, pagination))
    } else {
        Some((Some(oldest_commit), None))
    }
}

/// Fetch maintainers from recipe.yaml in a feedstock repo (fallback)
pub async fn fetch_recipe_maintainers(feedstock: &str) -> Result<Vec<String>> {
    let paths = ["recipe.yaml", "recipe/recipe.yaml"];

    for path in paths {
        let url = format!(
            "https://raw.githubusercontent.com/conda-forge/{}/main/{}",
            feedstock, path
        );

        let response = reqwest::get(&url).await;
        if let Ok(resp) = response {
            if resp.status().is_success() {
                if let Ok(content) = resp.text().await {
                    // Parse YAML to extract maintainers
                    if let Some(maintainers) = extract_maintainers_from_yaml(&content) {
                        if !maintainers.is_empty() {
                            return Ok(maintainers);
                        }
                    }
                }
            }
        }
    }

    Ok(vec![])
}

/// Extract maintainers from recipe.yaml content
fn extract_maintainers_from_yaml(content: &str) -> Option<Vec<String>> {
    // Simple regex-based extraction to avoid adding serde_yaml dependency
    // Looking for:
    // extra:
    //   recipe-maintainers:
    //     - user1
    //     - user2
    let mut in_extra = false;
    let mut in_maintainers = false;
    let mut maintainers = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "extra:" || trimmed.starts_with("extra:") {
            in_extra = true;
            continue;
        }

        if in_extra
            && (trimmed == "recipe-maintainers:" || trimmed.starts_with("recipe-maintainers:"))
        {
            in_maintainers = true;
            continue;
        }

        if in_maintainers {
            if trimmed.starts_with("- ") {
                let name = trimmed.trim_start_matches("- ").trim();
                if !name.is_empty() {
                    maintainers.push(name.to_string());
                }
            } else if !trimmed.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                // End of maintainers section
                break;
            }
        }

        // Reset if we hit a new top-level key
        if !line.starts_with(' ') && !line.starts_with('\t') && trimmed.ends_with(':') {
            in_extra = trimmed == "extra:";
            in_maintainers = false;
        }
    }

    Some(maintainers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_maintainers() {
        let yaml = r#"
package:
  name: test

extra:
  recipe-maintainers:
    - user1
    - user2
    - user3
"#;
        let maintainers = extract_maintainers_from_yaml(yaml).unwrap();
        assert_eq!(maintainers, vec!["user1", "user2", "user3"]);
    }
}
