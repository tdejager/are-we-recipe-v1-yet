use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct Repository {
    name: String,
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    total_count: u32,
    items: Vec<Repository>,
}

#[derive(Debug, Deserialize)]
struct TreeResponse {
    tree: Vec<TreeItem>,
}

#[derive(Debug, Deserialize)]
struct TreeItem {
    path: String,
    #[serde(rename = "type")]
    item_type: String,
}

#[derive(Debug, Serialize)]
struct FeedstockStats {
    total_feedstocks: u32,
    recipe_v1_count: u32,
    meta_yaml_count: u32,
    unknown_count: u32,
    last_updated: String,
}

const GITHUB_API_BASE: &str = "https://api.github.com";
const RATE_LIMIT_DELAY: Duration = Duration::from_millis(100);

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
    let mut total_feedstocks = 0;
    let mut recipe_v1_count = 0;
    let mut meta_yaml_count = 0;
    let mut unknown_count = 0;

    let mut page = 1;
    let per_page = 100;

    loop {
        println!("🔍 Fetching feedstocks page {}...", page);

        let search_url = format!(
            "{}/search/repositories?q=org:conda-forge+fork:false+{}+in:name&per_page={}&page={}",
            GITHUB_API_BASE, "-feedstock", per_page, page
        );

        let response = client
            .get(&search_url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "conda-forge-tracker")
            .send()
            .await
            .context("Failed to fetch feedstocks from GitHub")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }

        let search_result: SearchResponse = response
            .json()
            .await
            .context("Failed to parse search response")?;

        if search_result.items.is_empty() {
            break;
        }

        total_feedstocks += search_result.items.len() as u32;

        // Check each feedstock for recipe type
        for repo in &search_result.items {
            if repo.name.ends_with("-feedstock") {
                let recipe_type = check_recipe_type(client, token, &repo.full_name).await?;
                match recipe_type {
                    RecipeType::RecipeV1 => recipe_v1_count += 1,
                    RecipeType::MetaYaml => meta_yaml_count += 1,
                    RecipeType::Unknown => unknown_count += 1,
                }

                // Rate limiting
                sleep(RATE_LIMIT_DELAY).await;
            }
        }

        // If we got fewer results than requested, we're on the last page
        if (search_result.items.len() as u32) < per_page {
            break;
        }

        page += 1;
    }

    Ok(FeedstockStats {
        total_feedstocks,
        recipe_v1_count,
        meta_yaml_count,
        unknown_count,
        last_updated: chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(Debug)]
enum RecipeType {
    RecipeV1, // Has recipe.yaml
    MetaYaml, // Has meta.yaml
    Unknown,  // Neither or both
}

async fn check_recipe_type(
    client: &Client,
    token: &str,
    repo_full_name: &str,
) -> Result<RecipeType> {
    let tree_url = format!(
        "{}/repos/{}/git/trees/main?recursive=1",
        GITHUB_API_BASE, repo_full_name
    );

    let response = client
        .get(&tree_url)
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "conda-forge-tracker")
        .send()
        .await;

    let response = match response {
        Ok(r) if r.status().is_success() => r,
        _ => return Ok(RecipeType::Unknown), // Skip repos we can't access
    };

    let tree_result: TreeResponse = match response.json().await {
        Ok(r) => r,
        Err(_) => return Ok(RecipeType::Unknown),
    };

    let mut has_recipe_yaml = false;
    let mut has_meta_yaml = false;

    for item in &tree_result.tree {
        if item.path == "recipe/recipe.yaml" {
            has_recipe_yaml = true;
        } else if item.path == "recipe/meta.yaml" {
            has_meta_yaml = true;
        }
    }

    match (has_recipe_yaml, has_meta_yaml) {
        (true, false) => Ok(RecipeType::RecipeV1),
        (false, true) => Ok(RecipeType::MetaYaml),
        _ => Ok(RecipeType::Unknown),
    }
}
