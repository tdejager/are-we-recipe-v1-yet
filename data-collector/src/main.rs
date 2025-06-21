use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct OrganizationData {
    organization: Organization,
}

#[derive(Debug, Deserialize)]
struct Organization {
    repositories: RepositoryConnection,
}

#[derive(Debug, Deserialize)]
struct RepositoryConnection {
    #[serde(rename = "totalCount")]
    total_count: u32,
    nodes: Vec<Repository>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Repository {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RepositoryFileData {
    repository: Option<RepositoryObject>,
}

#[derive(Debug, Deserialize)]
struct RepositoryObject {
    object: Option<GitObject>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum GitObject {
    Tree { entries: Vec<TreeEntry> },
}

#[derive(Debug, Deserialize)]
struct TreeEntry {
    name: String,
    #[serde(rename = "type")]
    entry_type: String,
}

#[derive(Debug, Deserialize)]
struct SearchData {
    search: SearchResult,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    #[serde(rename = "repositoryCount")]
    repository_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedFeedstockList {
    feedstocks: Vec<Repository>,
    cached_at: String,
    total_count: u32,
}

#[derive(Debug, Serialize)]
struct FeedstockStats {
    total_feedstocks: u32,
    recipe_v1_count: u32,
    meta_yaml_count: u32,
    unknown_count: u32,
    last_updated: String,
}

const GITHUB_GRAPHQL_API: &str = "https://api.github.com/graphql";
const RATE_LIMIT_DELAY: Duration = Duration::from_millis(10);
const FEEDSTOCK_CACHE_FILE: &str = "feedstock-list.toml";
const CACHE_DURATION_DAYS: i64 = 5;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting conda-forge feedstock analysis...");

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let github_token =
        env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable is required")?;

    let client = Client::new();
    let stats = collect_feedstock_stats(&client, &github_token).await?;

    // Write to TOML file
    let toml_content =
        toml::to_string_pretty(&stats).context("Failed to serialize stats to TOML")?;

    fs::write("feedstock-stats.toml", toml_content)
        .context("Failed to write feedstock-stats.toml")?;

    println!("✅ Analysis complete!");
    println!("📊 Total feedstocks: {}", stats.total_feedstocks);
    println!("📝 Recipe v1 (recipe.yaml): {}", stats.recipe_v1_count);
    println!("📄 Legacy (meta.yaml): {}", stats.meta_yaml_count);
    println!("❓ Unknown/Other: {}", stats.unknown_count);
    println!("💾 Results saved to feedstock-stats.toml");

    Ok(())
}

async fn collect_feedstock_stats(client: &Client, token: &str) -> Result<FeedstockStats> {
    // Try to load from cache first
    let (all_feedstocks, total_feedstocks) = if let Some(cached) = load_cached_feedstocks()? {
        println!("🚀 Using cached feedstock list ({} feedstocks)", cached.feedstocks.len());
        (cached.feedstocks, cached.total_count)
    } else {
        // Get total count of feedstocks using efficient search query
        println!("📋 Getting total feedstock count...");
        let total_count = get_feedstock_count(client, token).await?;
        println!("✅ Found {} total feedstocks", total_count);

        // Collect all feedstock repositories
        println!("📋 Collecting all feedstock repositories...");
        let feedstocks = collect_all_feedstocks(client, token, total_count).await?;
        
        // Save to cache
        save_feedstocks_to_cache(&feedstocks, total_count)?;
        
        (feedstocks, total_count)
    };
    println!("🔬 Analyzing recipe types...");

    let mut recipe_v1_count = 0;
    let mut meta_yaml_count = 0;
    let mut unknown_count = 0;
    let mut processed = 0;

    // Check each feedstock for recipe type
    for repo in &all_feedstocks {
        processed += 1;
        print!(
            "📊 Progress: {}/{} ({:.1}%) - Checking {}...",
            processed,
            total_feedstocks,
            (processed as f32 / total_feedstocks as f32) * 100.0,
            repo.name
        );

        let recipe_type = check_recipe_type(client, token, &repo.name).await?;
        match recipe_type {
            RecipeType::RecipeV1 => {
                recipe_v1_count += 1;
                println!(" ✨ Recipe v1");
            }
            RecipeType::MetaYaml => {
                meta_yaml_count += 1;
                println!(" 📄 meta.yaml");
            }
            RecipeType::Unknown => {
                unknown_count += 1;
                println!(" ❓ Unknown");
            }
        }

        // Rate limiting
        sleep(RATE_LIMIT_DELAY).await;
    }

    Ok(FeedstockStats {
        total_feedstocks,
        recipe_v1_count,
        meta_yaml_count,
        unknown_count,
        last_updated: chrono::Utc::now().to_rfc3339(),
    })
}

async fn collect_all_feedstocks(
    client: &Client,
    token: &str,
    total_feedstocks: u32,
) -> Result<Vec<Repository>> {
    let mut all_feedstocks = Vec::new();
    let mut cursor: Option<String> = None;
    let mut page = 1;
    let mut feedstocks_left = total_feedstocks;

    loop {
        println!("🔍 Fetching conda-forge repositories page {}...", page);

        let query = json!({
            "query": r#"
                query($cursor: String) {
                    organization(login: "conda-forge") {
                        repositories(first: 100, after: $cursor) {
                            totalCount
                            nodes {
                                name
                            }
                            pageInfo {
                                hasNextPage
                                endCursor
                            }
                        }
                    }
                }
            "#,
            "variables": {
                "cursor": cursor
            }
        });

        let response = client
            .post(GITHUB_GRAPHQL_API)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "conda-forge-tracker")
            .json(&query)
            .send()
            .await
            .context("Failed to fetch repositories from GitHub GraphQL API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "GitHub GraphQL API error: {}",
                response.status()
            ));
        }

        let graphql_response: GraphQLResponse<OrganizationData> = response
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        if let Some(errors) = graphql_response.errors {
            return Err(anyhow::anyhow!("GraphQL errors: {:?}", errors));
        }

        let data = graphql_response
            .data
            .context("No data in GraphQL response")?;
        let repos = data.organization.repositories;

        if page == 1 {
            println!("📊 Total conda-forge repositories: {}", repos.total_count);
        }

        // Filter for feedstocks only
        let feedstocks: Vec<Repository> = repos
            .nodes
            .into_iter()
            .filter(|repo| repo.name.ends_with("-feedstock"))
            .collect();

        feedstocks_left -= feedstocks.len() as u32;
        println!(
            "   Found {} feedstocks on this page, {} left",
            feedstocks.len(),
            feedstocks_left
        );
        all_feedstocks.extend(feedstocks);

        if !repos.page_info.has_next_page {
            break;
        }

        cursor = repos.page_info.end_cursor;
        page += 1;

        // Rate limiting
        sleep(RATE_LIMIT_DELAY).await;
    }

    Ok(all_feedstocks)
}

async fn get_feedstock_count(client: &Client, token: &str) -> Result<u32> {
    let query = json!({
        "query": r#"
            query {
                search(query: "org:conda-forge feedstock in:name", type: REPOSITORY, first: 1) {
                    repositoryCount
                }
            }
        "#
    });

    let response = client
        .post(GITHUB_GRAPHQL_API)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "conda-forge-tracker")
        .json(&query)
        .send()
        .await
        .context("Failed to get feedstock count from GitHub GraphQL API")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "GitHub GraphQL API error: {}",
            response.status()
        ));
    }

    let graphql_response: GraphQLResponse<SearchData> = response
        .json()
        .await
        .context("Failed to parse search GraphQL response")?;

    if let Some(errors) = graphql_response.errors {
        return Err(anyhow::anyhow!("GraphQL errors: {:?}", errors));
    }

    let data = graphql_response
        .data
        .context("No data in search GraphQL response")?;
    Ok(data.search.repository_count)
}

fn is_cache_fresh(cache_file: &str) -> Result<bool> {
    let metadata = match fs::metadata(cache_file) {
        Ok(m) => m,
        Err(_) => return Ok(false), // No cache file exists
    };

    let modified = metadata.modified().context("Failed to get cache file modification time")?;
    let cache_age = std::time::SystemTime::now().duration_since(modified)?;
    let max_age = Duration::from_secs(CACHE_DURATION_DAYS as u64 * 24 * 60 * 60);

    Ok(cache_age < max_age)
}

fn load_cached_feedstocks() -> Result<Option<CachedFeedstockList>> {
    if !is_cache_fresh(FEEDSTOCK_CACHE_FILE)? {
        println!("📅 Cache is older than {} days, will refresh", CACHE_DURATION_DAYS);
        return Ok(None);
    }

    println!("📂 Loading feedstocks from cache...");
    let content = match fs::read_to_string(FEEDSTOCK_CACHE_FILE) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let cached: CachedFeedstockList = toml::from_str(&content)
        .context("Failed to parse cached feedstock list")?;

    println!("✅ Loaded {} feedstocks from cache", cached.feedstocks.len());
    Ok(Some(cached))
}

fn save_feedstocks_to_cache(feedstocks: &[Repository], total_count: u32) -> Result<()> {
    let cached = CachedFeedstockList {
        feedstocks: feedstocks.to_vec(),
        cached_at: chrono::Utc::now().to_rfc3339(),
        total_count,
    };

    let toml_content = toml::to_string_pretty(&cached)
        .context("Failed to serialize feedstock list to TOML")?;

    fs::write(FEEDSTOCK_CACHE_FILE, toml_content)
        .context("Failed to write feedstock cache file")?;

    println!("💾 Cached {} feedstocks to {}", feedstocks.len(), FEEDSTOCK_CACHE_FILE);
    Ok(())
}

#[derive(Debug)]
enum RecipeType {
    RecipeV1, // Has recipe.yaml
    MetaYaml, // Has meta.yaml
    Unknown,  // Neither or both
}

async fn check_recipe_type(client: &Client, token: &str, repo_name: &str) -> Result<RecipeType> {
    let query = json!({
        "query": r#"
            query($owner: String!, $name: String!) {
                repository(owner: $owner, name: $name) {
                    object(expression: "HEAD:recipe") {
                        __typename
                        ... on Tree {
                            entries {
                                name
                                type
                            }
                        }
                    }
                }
            }
        "#,
        "variables": {
            "owner": "conda-forge",
            "name": repo_name
        }
    });

    let response = client
        .post(GITHUB_GRAPHQL_API)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "conda-forge-tracker")
        .json(&query)
        .send()
        .await;

    let response = match response {
        Ok(r) if r.status().is_success() => r,
        _ => return Ok(RecipeType::Unknown), // Skip repos we can't access
    };

    let graphql_response: GraphQLResponse<RepositoryFileData> = match response.json().await {
        Ok(r) => r,
        Err(_) => return Ok(RecipeType::Unknown),
    };

    if let Some(errors) = graphql_response.errors {
        // Some repos might not have recipe directory or might be private
        return Ok(RecipeType::Unknown);
    }

    let data = match graphql_response.data {
        Some(d) => d,
        None => return Ok(RecipeType::Unknown),
    };

    let repo = match data.repository {
        Some(r) => r,
        None => return Ok(RecipeType::Unknown),
    };

    let object = match repo.object {
        Some(o) => o,
        None => return Ok(RecipeType::Unknown),
    };

    let entries = match object {
        GitObject::Tree { entries } => entries,
    };

    let mut has_recipe_yaml = false;
    let mut has_meta_yaml = false;

    for entry in &entries {
        if entry.name == "recipe.yaml" {
            has_recipe_yaml = true;
        } else if entry.name == "meta.yaml" {
            has_meta_yaml = true;
        }
    }

    match (has_recipe_yaml, has_meta_yaml) {
        (true, false) => Ok(RecipeType::RecipeV1),
        (false, true) => Ok(RecipeType::MetaYaml),
        _ => Ok(RecipeType::Unknown),
    }
}
