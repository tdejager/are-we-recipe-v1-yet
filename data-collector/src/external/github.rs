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
#[derive(Debug)]
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

        for chunk in feedstocks.chunks(BATCH_SIZE) {
            let query = build_batch_query(chunk);
            let response = self.execute_query(&query).await?;
            let results = parse_batch_response(chunk, &response)?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    async fn execute_query(&self, query: &str) -> Result<serde_json::Value> {
        let response = self
            .client
            .post(GITHUB_GRAPHQL_URL)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?;

        let status = response.status();
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

        result.data.context("No data in GraphQL response")
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
                            historyMain: history(first: 1, path: "recipe.yaml") {{
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
                            historyAlt: history(first: 1, path: "recipe/recipe.yaml") {{
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

/// Parse the batched response and extract commit information
fn parse_batch_response(
    feedstocks: &[String],
    response: &serde_json::Value,
) -> Result<Vec<RecipeHistoryResult>> {
    let mut results = Vec::new();

    for (i, feedstock) in feedstocks.iter().enumerate() {
        let repo_key = format!("repo{}", i);
        let repo_data = response.get(&repo_key);

        let result = match repo_data {
            Some(repo) if !repo.is_null() => {
                // Try main path first, then alt path
                let commit = extract_first_commit(repo, "historyMain")
                    .or_else(|| extract_first_commit(repo, "historyAlt"));

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

    Ok(results)
}

/// Extract the first commit from the history
fn extract_first_commit(repo: &serde_json::Value, history_key: &str) -> Option<FirstRecipeCommit> {
    let nodes = repo
        .get("defaultBranchRef")?
        .get("target")?
        .get(history_key)?
        .get("nodes")?
        .as_array()?;

    let commit = nodes.first()?;
    let author = commit.get("author")?;

    Some(FirstRecipeCommit {
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
    })
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
