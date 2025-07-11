use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize)]
struct FeedstockStats {
    total_feedstocks: u32,
    recipe_v1_count: u32,
    meta_yaml_count: u32,
    unknown_count: u32,
    last_updated: String,
    #[serde(default)]
    feedstock_states: BTreeMap<String, FeedstockEntry>,
    #[serde(default)]
    top_unconverted_by_downloads: Vec<TopFeedstock>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FeedstockEntry {
    recipe_type: RecipeType,
    last_changed: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TopFeedstock {
    name: String,
    downloads: u64,
    recipe_type: RecipeType,
}

const CF_GRAPH_REPO_URL: &str = "https://github.com/regro/cf-graph-countyfair.git";
const CF_GRAPH_LOCAL_PATH: &str = "../cf-graph-countyfair";

#[derive(Debug, Deserialize)]
struct NodeAttrsJson {
    feedstock_name: String,
    #[serde(rename = "conda-forge.yml", default)]
    conda_forge_yml: Option<CondaForgeYml>,
}

#[derive(Debug, Deserialize)]
struct CondaForgeYml {
    #[serde(default)]
    conda_build_tool: Option<String>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Show detailed progress information
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze conda-forge feedstocks using cf-graph-countyfair data
    Analyze {
        /// Force re-clone the repository even if it exists
        #[arg(long)]
        force_clone: bool,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum RecipeType {
    #[serde(rename = "recipe_v1")]
    RecipeV1, // Has recipe.yaml
    #[serde(rename = "meta_yaml")]
    MetaYaml, // Has meta.yaml
    #[serde(rename = "unknown")]
    Unknown, // Neither or both
}

<<<<<<< Updated upstream
||||||| Stash base
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum ConversionCredit {
    Single(String),
    Multiple(Vec<String>),
}

impl ConversionCredit {
    fn from_single(email: String) -> Self {
        ConversionCredit::Single(email)
    }

    fn get_emails(&self) -> Vec<&String> {
        match self {
            ConversionCredit::Single(email) => vec![email],
            ConversionCredit::Multiple(emails) => emails.iter().collect(),
        }
    }
}


#[derive(Debug, Serialize)]
struct GraphQLQuery {
    query: String,
    variables: GraphQLVariables,
}

#[derive(Debug, Serialize)]
struct GraphQLVariables {
    owner: String,
    name: String,
    oid: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    repository: Option<GraphQLRepository>,
}

#[derive(Debug, Deserialize)]
struct GraphQLRepository {
    object: Option<GraphQLCommit>,
}

#[derive(Debug, Deserialize)]
struct GraphQLCommit {
    author: GraphQLAuthor,
}

#[derive(Debug, Deserialize)]
struct GraphQLAuthor {
    user: Option<GraphQLUser>,
}

#[derive(Debug, Deserialize)]
struct GraphQLUser {
    login: String,
}

=======
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum ConversionCredit {
    Single(String),
    Multiple(Vec<String>),
}

impl ConversionCredit {
    fn from_single(email: String) -> Self {
        ConversionCredit::Single(email)
    }

    fn get_emails(&self) -> Vec<&String> {
        match self {
            ConversionCredit::Single(email) => vec![email],
            ConversionCredit::Multiple(emails) => emails.iter().collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct GraphQLQuery {
    query: String,
    variables: GraphQLVariables,
}

#[derive(Debug, Serialize)]
struct GraphQLVariables {
    owner: String,
    name: String,
    oid: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    repository: Option<GraphQLRepository>,
}

#[derive(Debug, Deserialize)]
struct GraphQLRepository {
    object: Option<GraphQLCommit>,
}

#[derive(Debug, Deserialize)]
struct GraphQLCommit {
    author: GraphQLAuthor,
}

#[derive(Debug, Deserialize)]
struct GraphQLAuthor {
    user: Option<GraphQLUser>,
}

#[derive(Debug, Deserialize)]
struct GraphQLUser {
    login: String,
}

>>>>>>> Stashed changes
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
    let cli = Cli::parse();

    println!("🚀 Starting conda-forge feedstock analysis...");

    let stats = match cli.command {
        Some(Commands::Analyze { force_clone }) => {
            collect_stats_from_node_attrs(force_clone, cli.verbose).await?
        }
        None => collect_stats_from_node_attrs(false, cli.verbose).await?,
    };

    // Write to TOML file
    let toml_content =
        toml::to_string_pretty(&stats).context("Failed to serialize stats to TOML")?;

    let path = std::env::var("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR not set")?;
    fs::write(format!("{}/../feedstock-stats.toml", path), toml_content)
        .context("Failed to write feedstock-stats.toml")?;

    // Clean up sparse checkout repository
    cleanup_sparse_checkout_repo(cli.verbose)?;

    println!("✅ Analysis complete!");
    println!("📊 Total feedstocks: {}", stats.total_feedstocks);
    println!("📝 Recipe v1 (recipe.yaml): {}", stats.recipe_v1_count);
    println!("📄 Legacy (meta.yaml): {}", stats.meta_yaml_count);
    println!("❓ Unknown/Other: {}", stats.unknown_count);
    println!("💾 Results saved to feedstock-stats.toml");

    Ok(())
}

fn load_existing_stats_if_exists() -> Option<FeedstockStats> {
    let path = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let stats_file = format!("{}/../feedstock-stats.toml", path);
    println!("🔍 Looking for existing stats at: {}", stats_file);
    let content = fs::read_to_string(&stats_file).ok()?;
    let stats: FeedstockStats = toml::from_str(&content).ok()?;
    println!(
        "📂 Loaded existing stats: {} total feedstocks, {} feedstock_states entries",
        stats.total_feedstocks, stats.feedstock_states.len()
    );
    Some(stats)
}

async fn collect_stats_from_node_attrs(force_reload: bool, verbose: bool) -> Result<FeedstockStats> {
    // Load existing stats for historical comparison
    let existing_stats = load_existing_stats_if_exists();

    // Fetch download counts
    println!("📥 Fetching download counts from Google Cloud Storage...");
    let download_counts = fetch_download_counts().await?;
    println!("📊 Fetched {} download counts", download_counts.len());

    // Set up sparse checkout repository
    ensure_sparse_checkout_repo(force_reload, verbose)?;

    println!("📂 Scanning node_attrs directory...");
    let node_attrs_path = format!("{}/node_attrs", CF_GRAPH_LOCAL_PATH);

    if !Path::new(&node_attrs_path).exists() {
        return Err(anyhow::anyhow!(
            "node_attrs directory not found at {}",
            node_attrs_path
        ));
    }

    // Count total JSON files first for progress bar
    let json_files: Vec<_> = WalkDir::new(&node_attrs_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().map_or(false, |ext| ext == "json")
        })
        .collect();

    let total_files = json_files.len();
    println!("📊 Found {} JSON files to analyze", total_files);

    // Set up progress bar
    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap(),
    );

    let mut feedstock_states = BTreeMap::new();
    let current_time = Utc::now().to_rfc3339();
    let mut processed = 0;

    // Process each JSON file
    for entry in json_files {
        match parse_node_attrs_file(entry.path()) {
            Ok(node_data) => {
                let feedstock_name = format!("{}-feedstock", node_data.feedstock_name);
                let recipe_type = determine_recipe_type_from_node(&node_data);

                // Timestamp logic:
                // 1. New feedstock -> use current timestamp
                // 2. Existing feedstock, no conversion -> keep existing timestamp  
                // 3. Existing feedstock converted to RecipeV1 -> use current timestamp
                let last_changed = if let Some(ref existing) = existing_stats {
                    if let Some(existing_entry) = existing.feedstock_states.get(&feedstock_name) {
                        // Feedstock already exists - only update if converted to RecipeV1
                        if existing_entry.recipe_type != RecipeType::RecipeV1 
                            && recipe_type == RecipeType::RecipeV1 {
                            if verbose {
                                println!("🔄 CONVERTED: {} from {:?} to {:?}", feedstock_name, existing_entry.recipe_type, recipe_type);
                            }
                            current_time.clone() // Converted to RecipeV1, update timestamp
                        } else {
                            if verbose && processed < 5 {
                                println!("📌 KEEPING: {} - {:?} (old: {}, keeping: {})", 
                                    feedstock_name, recipe_type, current_time, existing_entry.last_changed);
                            }
                            existing_entry.last_changed.clone() // No conversion, keep existing timestamp
                        }
                    } else {
                        if verbose && processed < 5 {
                            println!("🆕 NEW: {} - {:?}", feedstock_name, recipe_type);
                        }
                        current_time.clone() // New feedstock, use current timestamp
                    }
                } else {
                    current_time.clone() // First run, use current timestamp
                };

                feedstock_states.insert(feedstock_name, FeedstockEntry {
                    recipe_type,
                    last_changed,
                });
                processed += 1;

                if verbose && processed % 1000 == 0 {
                    pb.println(format!("📊 Processed {} feedstocks...", processed));
                }
            }
            Err(_) => {
                // Skip files that can't be parsed (might not be feedstock files)
                continue;
            }
        }
        pb.inc(1);
    }

    pb.finish_with_message("✅ Analysis complete!");
    println!("📈 Processed {} total feedstocks", processed);

    // Calculate counts from the HashMap
    let recipe_v1_count = feedstock_states
        .values()
        .filter(|entry| entry.recipe_type == RecipeType::RecipeV1)
        .count() as u32;
    let meta_yaml_count = feedstock_states
        .values()
        .filter(|entry| entry.recipe_type == RecipeType::MetaYaml)
        .count() as u32;
    let unknown_count = feedstock_states
        .values()
        .filter(|entry| entry.recipe_type == RecipeType::Unknown)
        .count() as u32;
    let total_feedstocks = processed;

    println!(
        "📝 Recipe v1 (rattler-build + schema_version=1): {}",
        recipe_v1_count
    );
    println!("📄 Legacy (conda-build or other): {}", meta_yaml_count);
    println!("❓ Unknown/Other: {}", unknown_count);

    // Find newly converted feedstocks
    let newly_converted = if let Some(ref existing) = existing_stats {
        feedstock_states
            .iter()
            .filter(|(name, entry)| {
                entry.recipe_type == RecipeType::RecipeV1
                    && existing
                        .feedstock_states
                        .get(*name)
                        .map_or(true, |old_entry| old_entry.recipe_type != RecipeType::RecipeV1)
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    if !newly_converted.is_empty() {
        println!("🎉 {} newly converted to Recipe v1!", newly_converted.len());
        if verbose {
            for feedstock in &newly_converted {
                println!("  ✨ {}", feedstock);
            }
        }
    }

    // Calculate top unconverted feedstocks by downloads
    let top_unconverted = calculate_top_unconverted_feedstocks(&feedstock_states, &download_counts, 50);
    println!("🏆 Found {} top unconverted feedstocks by downloads", top_unconverted.len());

    Ok(FeedstockStats {
        total_feedstocks,
        recipe_v1_count,
        meta_yaml_count,
        unknown_count,
        last_updated: Utc::now().to_rfc3339(),
        feedstock_states,
        top_unconverted_by_downloads: top_unconverted,
    })
}

fn ensure_sparse_checkout_repo(force_reload: bool, verbose: bool) -> Result<()> {
    let repo_path = Path::new(CF_GRAPH_LOCAL_PATH);

    if force_reload && repo_path.exists() {
        println!("🗑️  Removing existing repository for fresh sparse checkout...");
        fs::remove_dir_all(repo_path).context("Failed to remove existing repository")?;
    }

    if !repo_path.exists() {
        println!("📥 Creating sparse checkout of cf-graph-countyfair repository...");
        println!("🎯 Only downloading node_attrs directory (much faster than full clone)");

        // Create directory and initialize git
        fs::create_dir_all(repo_path).context("Failed to create repository directory")?;

        let init_result = Command::new("git")
            .current_dir(repo_path)
            .arg("init")
            .output()
            .context("Failed to run git init")?;

        if !init_result.status.success() {
            return Err(anyhow::anyhow!(
                "git init failed: {}",
                String::from_utf8_lossy(&init_result.stderr)
            ));
        }

        if verbose {
            println!("✅ Git repository initialized");
        }

        // Add remote
        let remote_result = Command::new("git")
            .current_dir(repo_path)
            .args(&["remote", "add", "origin", CF_GRAPH_REPO_URL])
            .output()
            .context("Failed to add remote")?;

        if !remote_result.status.success() {
            return Err(anyhow::anyhow!(
                "git remote add failed: {}",
                String::from_utf8_lossy(&remote_result.stderr)
            ));
        }

        if verbose {
            println!("✅ Remote added");
        }

        // Enable sparse checkout
        let sparse_config_result = Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "core.sparseCheckout", "true"])
            .output()
            .context("Failed to enable sparse checkout")?;

        if !sparse_config_result.status.success() {
            return Err(anyhow::anyhow!(
                "git config core.sparseCheckout failed: {}",
                String::from_utf8_lossy(&sparse_config_result.stderr)
            ));
        }

        if verbose {
            println!("✅ Sparse checkout enabled");
        }

        // Set sparse checkout patterns
        let sparse_checkout_path = repo_path.join(".git/info/sparse-checkout");
        fs::write(&sparse_checkout_path, "node_attrs/*\n")
            .context("Failed to write sparse-checkout file")?;

        if verbose {
            println!("✅ Sparse checkout pattern set to node_attrs/*");
        }

        // Pull with depth=1
        let pull_result = Command::new("git")
            .current_dir(repo_path)
            .args(&["pull", "origin", "master", "--depth=1"])
            .output()
            .context("Failed to pull repository")?;

        if !pull_result.status.success() {
            return Err(anyhow::anyhow!(
                "git pull failed: {}",
                String::from_utf8_lossy(&pull_result.stderr)
            ));
        }

        println!("✅ Sparse checkout completed successfully");

        if verbose {
            println!("📁 Repository structure:");
            let ls_result = Command::new("ls")
                .current_dir(repo_path)
                .args(&["-la"])
                .output()
                .context("Failed to list directory contents")?;

            if ls_result.status.success() {
                println!("{}", String::from_utf8_lossy(&ls_result.stdout));
            }
        }
    } else {
        // Check if existing sparse checkout is valid
        let node_attrs_path = repo_path.join("node_attrs");
        if node_attrs_path.exists() {
            if verbose {
                println!("📂 Using existing sparse checkout");
            }
            return Ok(());
        } else {
            println!("📂 Existing sparse checkout incomplete, recreating...");
            fs::remove_dir_all(repo_path).context("Failed to remove existing repository")?;
            return ensure_sparse_checkout_repo(false, verbose); // Recursive call to re-create fresh
        }
    }

    Ok(())
}

fn parse_node_attrs_file(path: &Path) -> Result<NodeAttrsJson> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?;

    let node_data: NodeAttrsJson = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON in file: {:?}", path))?;

    Ok(node_data)
}

fn cleanup_sparse_checkout_repo(verbose: bool) -> Result<()> {
    let repo_path = Path::new(CF_GRAPH_LOCAL_PATH);
    
    if repo_path.exists() {
        if verbose {
            println!("🗑️  Cleaning up sparse checkout repository...");
        }
        fs::remove_dir_all(repo_path).context("Failed to remove sparse checkout repository")?;
        if verbose {
            println!("✅ Sparse checkout repository cleaned up");
        }
    }
    
    Ok(())
}

fn determine_recipe_type_from_node(node_data: &NodeAttrsJson) -> RecipeType {
    // Check if conda_build_tool is set to rattler-build in conda-forge.yml
    if let Some(conda_forge_yml) = &node_data.conda_forge_yml {
        if let Some(conda_build_tool) = &conda_forge_yml.conda_build_tool {
            if conda_build_tool == "rattler-build" {
                return RecipeType::RecipeV1;
            }
        }
    }

    // If no rattler-build conda_build_tool found, it's using conda-build (legacy)
    RecipeType::MetaYaml
}

async fn fetch_download_counts() -> Result<HashMap<String, u64>> {
    let url = "https://storage.googleapis.com/download-count-cache/top_downloads_conda-forge.json";
    let client = reqwest::Client::new();
    
    let response = client.get(url).send().await
        .context("Failed to fetch download counts")?;
    
    let download_data: Vec<[serde_json::Value; 2]> = response.json().await
        .context("Failed to parse download counts JSON")?;
    
    let mut download_counts = HashMap::new();
    
    for entry in download_data {
        if let (Some(package_name), Some(count)) = (entry[0].as_str(), entry[1].as_u64()) {
            // Convert package name to feedstock name format with special mappings
            let feedstock_name = match package_name {
                "libzlib" => "zlib-feedstock".to_string(),
                "libblas" => "blas-feedstock".to_string(), 
                _ => format!("{}-feedstock", package_name)
            };
            
            // Only insert if this is a higher count or the feedstock doesn't exist yet
            // This prioritizes libzlib over zlib if both map to zlib-feedstock
            if let Some(&existing_count) = download_counts.get(&feedstock_name) {
                if count > existing_count {
                    download_counts.insert(feedstock_name, count);
                }
            } else {
                download_counts.insert(feedstock_name, count);
            }
        }
    }
    
    Ok(download_counts)
}

<<<<<<< Updated upstream
||||||| Stash base
async fn generate_leaderboard(
    limit: usize,
    test_limit: Option<usize>,
    verbose: bool,
) -> Result<()> {
    println!("🏆 Generating contributor leaderboard for Recipe v1 conversions...");
    println!("📊 Showing top {} contributors", limit);

    // Load existing stats
    let mut stats = load_existing_stats_if_exists()
        .ok_or_else(|| anyhow::anyhow!("No existing stats found. Run 'stats' command first."))?;

    // Find Recipe v1 feedstocks that need conversion credit analysis
    let mut unanalyzed_feedstocks: Vec<String> = stats
        .feedstock_states
        .iter()
        .filter(|(_, entry)| {
            entry.recipe_type == RecipeType::RecipeV1 && entry.conversion_credit.is_none()
        })
        .map(|(name, _)| name.clone())
        .collect();

    // Apply test limit if specified
    if let Some(limit) = test_limit {
        unanalyzed_feedstocks.truncate(limit);
        println!("🧪 Test mode: limiting analysis to {} feedstocks", limit);
    }

    if unanalyzed_feedstocks.is_empty() {
        println!("✅ All Recipe v1 feedstocks already have conversion credits");
    } else {
        println!(
            "🔍 Found {} Recipe v1 feedstocks to analyze for conversion credits",
            unanalyzed_feedstocks.len()
        );

        // Create HTTP client for GitHub API calls
        let client = reqwest::Client::new();

        // Add progress bar for large numbers of feedstocks
        let pb = if unanalyzed_feedstocks.len() > 10 {
            let pb = ProgressBar::new(unanalyzed_feedstocks.len() as u64);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
                ).unwrap(),
            );
            Some(pb)
        } else {
            None
        };

        // Process each unanalyzed feedstock
        for (index, feedstock_name) in unanalyzed_feedstocks.iter().enumerate() {
            if let Some(ref pb) = pb {
                pb.set_message(feedstock_name.clone());
            }

            if verbose || index % 50 == 0 {
                println!(
                    "📦 [{}/{}] Analyzing {}",
                    index + 1,
                    unanalyzed_feedstocks.len(),
                    feedstock_name
                );
            }

            match analyze_feedstock_conversion_credit(feedstock_name, verbose).await {
                Ok(Some((commit_sha, author_email))) => {
                    // Try to resolve GitHub username from commit
                    let github_username =
                        resolve_github_username_from_commit(feedstock_name, &commit_sha, &client)
                            .await
                            .unwrap_or(None);

                    // Fallback to noreply email parsing if GraphQL fails
                    let final_username =
                        github_username.or_else(|| resolve_github_username_fallback(&author_email));

                    let display_name = final_username
                        .map(|username| format!("@{}", username))
                        .unwrap_or(author_email);

                    // Update the feedstock entry with conversion credit
                    if let Some(entry) = stats.feedstock_states.get_mut(feedstock_name) {
                        entry.conversion_credit =
                            Some(ConversionCredit::from_single(display_name.clone()));
                        if verbose {
                            println!("✅ Credited to: {}", display_name);
                        }
                    }
                }
                Ok(None) => {
                    if verbose {
                        println!(
                            "⚠️  No recipe.yaml found in git history for {}",
                            feedstock_name
                        );
                    }
                }
                Err(e) => {
                    if verbose {
                        println!("❌ Failed to analyze {}: {}", feedstock_name, e);
                    }
                }
            }

            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        }

        if let Some(pb) = pb {
            pb.finish_with_message("✅ Conversion credit analysis complete!");
        }

        // Save updated stats
        let toml_content = toml::to_string_pretty(&stats)?;
        let path = std::env::var("CARGO_MANIFEST_DIR")?;
        fs::write(format!("{}/../feedstock-stats.toml", path), toml_content)?;
        println!("💾 Updated conversion credits saved to feedstock-stats.toml");
    }

    // Generate leaderboard from all credited conversions
    let mut conversion_counts: HashMap<String, u32> = HashMap::new();
    for (_, entry) in &stats.feedstock_states {
        if entry.recipe_type == RecipeType::RecipeV1 {
            if let Some(ref conversion_credit) = entry.conversion_credit {
                for email in conversion_credit.get_emails() {
                    *conversion_counts.entry(email.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    if conversion_counts.is_empty() {
        println!("📊 No conversion credits found yet");
        return Ok(());
    }

    // Sort by conversion count
    let mut leaderboard: Vec<(String, u32)> = conversion_counts.into_iter().collect();
    leaderboard.sort_by(|a, b| b.1.cmp(&a.1));

    // Display leaderboard (usernames are already resolved during collection)
    println!("\n🏆 Recipe v1 Conversion Leaderboard:");
    println!("=====================================");
    for (index, (contributor, count)) in leaderboard.iter().take(limit).enumerate() {
        println!("{}. {} - {} conversions", index + 1, contributor, count);
    }

    let total_credited = stats
        .feedstock_states
        .values()
        .filter(|entry| {
            entry.recipe_type == RecipeType::RecipeV1 && entry.conversion_credit.is_some()
        })
        .count();
    let total_recipe_v1 = stats
        .feedstock_states
        .values()
        .filter(|entry| entry.recipe_type == RecipeType::RecipeV1)
        .count();

    println!("\n📈 Statistics:");
    println!("Total Recipe v1 feedstocks: {}", total_recipe_v1);
    println!("Credited conversions: {}", total_credited);
    println!(
        "Uncredited conversions: {}",
        total_recipe_v1 - total_credited
    );

    // Save updated stats
    let toml_content = toml::to_string_pretty(&stats)?;
    let path = std::env::var("CARGO_MANIFEST_DIR")?;
    fs::write(format!("{}/../feedstock-stats.toml", path), toml_content)?;

    Ok(())
}

async fn analyze_feedstock_conversion_credit(
    feedstock_name: &str,
    _verbose: bool,
) -> Result<Option<(String, String)>> {
    let repo_url = format!("https://github.com/conda-forge/{}.git", feedstock_name);
    let temp_dir = std::env::temp_dir().join(format!("feedstock_analysis_{}", feedstock_name));

    // Clean up any existing temp directory
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    // Clone repository with shallow history
    let clone_result = Command::new("git")
        .args(&[
            "clone",
            "--depth=50", // Usually enough to find recipe.yaml addition
            &repo_url,
            temp_dir.to_str().unwrap(),
        ])
        .output()?;

    if !clone_result.status.success() {
        fs::remove_dir_all(&temp_dir).ok(); // Clean up on failure
        return Err(anyhow::anyhow!(
            "Failed to clone {}: {}",
            repo_url,
            String::from_utf8_lossy(&clone_result.stderr)
        ));
    }

    // Extract commit SHA and author email of commit that added recipe.yaml
    let git_log_result = Command::new("git")
        .current_dir(&temp_dir)
        .args(&[
            "log",
            "--follow",
            "--diff-filter=A",
            "--format=%H %ae",
            "-1",
            "--",
            "recipe/recipe.yaml",
        ])
        .output()?;

    // Clean up temp directory
    fs::remove_dir_all(&temp_dir).ok();

    if !git_log_result.status.success() {
        return Ok(None); // No recipe.yaml found in history
    }

    let output = String::from_utf8_lossy(&git_log_result.stdout);
    let trimmed_output = output.trim();
    if trimmed_output.is_empty() {
        return Ok(None);
    }

    // Parse "commit_sha author_email" format
    let parts: Vec<&str> = trimmed_output.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(None);
    }

    let commit_sha = parts[0].to_string();
    let author_email = parts[1].to_string();

    Ok(Some((commit_sha, author_email)))
}

async fn resolve_github_username_from_commit(
    feedstock_name: &str,
    commit_sha: &str,
    client: &reqwest::Client,
) -> Result<Option<String>> {
    let query = r#"
        query($owner: String!, $name: String!, $oid: GitObjectID!) {
            repository(owner: $owner, name: $name) {
                object(oid: $oid) {
                    ... on Commit {
                        author {
                            user {
                                login
                            }
                        }
                    }
                }
            }
        }
    "#;

    let graphql_query = GraphQLQuery {
        query: query.to_string(),
        variables: GraphQLVariables {
            owner: "conda-forge".to_string(),
            name: feedstock_name.to_string(),
            oid: commit_sha.to_string(),
        },
    };

    let response = client
        .post("https://api.github.com/graphql")
        .header("User-Agent", "conda-forge-leaderboard-tool")
        .header(
            "Authorization",
            &format!(
                "Bearer {}",
                std::env::var("GITHUB_TOKEN").unwrap_or_default()
            ),
        )
        .json(&graphql_query)
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let graphql_response: GraphQLResponse = response.json().await?;

    Ok(graphql_response
        .data
        .and_then(|data| data.repository)
        .and_then(|repo| repo.object)
        .and_then(|commit| commit.author.user)
        .map(|user| user.login))
}

fn resolve_github_username_fallback(email: &str) -> Option<String> {
    // Handle special GitHub email formats
    if email.contains("@users.noreply.github.com") {
        // Extract username from noreply email format: "123456+username@users.noreply.github.com"
        if let Some(at_pos) = email.find('@') {
            let local_part = &email[..at_pos];
            if let Some(plus_pos) = local_part.find('+') {
                let username = &local_part[plus_pos + 1..];
                return Some(username.to_string());
            }
        }
    }
    None
}

=======
async fn generate_leaderboard(
    limit: usize,
    test_limit: Option<usize>,
    verbose: bool,
) -> Result<()> {
    println!("🏆 Generating contributor leaderboard for Recipe v1 conversions...");
    println!("📊 Showing top {} contributors", limit);

    // Load existing stats
    let mut stats = load_existing_stats_if_exists()
        .ok_or_else(|| anyhow::anyhow!("No existing stats found. Run 'stats' command first."))?;

    // Find Recipe v1 feedstocks that need conversion credit analysis
    let mut unanalyzed_feedstocks: Vec<String> = stats
        .feedstock_states
        .iter()
        .filter(|(_, entry)| {
            entry.recipe_type == RecipeType::RecipeV1 && entry.conversion_credit.is_none()
        })
        .map(|(name, _)| name.clone())
        .collect();

    // Apply test limit if specified
    if let Some(limit) = test_limit {
        unanalyzed_feedstocks.truncate(limit);
        println!("🧪 Test mode: limiting analysis to {} feedstocks", limit);
    }

    if unanalyzed_feedstocks.is_empty() {
        println!("✅ All Recipe v1 feedstocks already have conversion credits");
    } else {
        println!(
            "🔍 Found {} Recipe v1 feedstocks to analyze for conversion credits",
            unanalyzed_feedstocks.len()
        );

        // Create HTTP client for GitHub API calls
        let client = reqwest::Client::new();

        // Add progress bar for large numbers of feedstocks
        let pb = if unanalyzed_feedstocks.len() > 10 {
            let pb = ProgressBar::new(unanalyzed_feedstocks.len() as u64);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
                ).unwrap(),
            );
            Some(pb)
        } else {
            None
        };

        // Process each unanalyzed feedstock
        for (index, feedstock_name) in unanalyzed_feedstocks.iter().enumerate() {
            if let Some(ref pb) = pb {
                pb.set_message(feedstock_name.clone());
            }

            if verbose || index % 50 == 0 {
                println!(
                    "📦 [{}/{}] Analyzing {}",
                    index + 1,
                    unanalyzed_feedstocks.len(),
                    feedstock_name
                );
            }

            match analyze_feedstock_conversion_credit(feedstock_name, verbose).await {
                Ok(Some((commit_sha, author_email))) => {
                    // Try to resolve GitHub username from commit
                    let github_username =
                        resolve_github_username_from_commit(feedstock_name, &commit_sha, &client)
                            .await
                            .unwrap_or(None);

                    // Fallback to noreply email parsing if GraphQL fails
                    let final_username =
                        github_username.or_else(|| resolve_github_username_fallback(&author_email));

                    let display_name = final_username
                        .map(|username| format!("@{}", username))
                        .unwrap_or(author_email);

                    // Update the feedstock entry with conversion credit
                    if let Some(entry) = stats.feedstock_states.get_mut(feedstock_name) {
                        entry.conversion_credit =
                            Some(ConversionCredit::from_single(display_name.clone()));
                        if verbose {
                            println!("✅ Credited to: {}", display_name);
                        }
                    }
                }
                Ok(None) => {
                    if verbose {
                        println!(
                            "⚠️  No recipe.yaml found in git history for {}",
                            feedstock_name
                        );
                    }
                }
                Err(e) => {
                    if verbose {
                        println!("❌ Failed to analyze {}: {}", feedstock_name, e);
                    }
                }
            }

            if let Some(ref pb) = pb {
                pb.inc(1);
            }
        }

        if let Some(pb) = pb {
            pb.finish_with_message("✅ Conversion credit analysis complete!");
        }

        // Save updated stats
        let toml_content = toml::to_string_pretty(&stats)?;
        let path = std::env::var("CARGO_MANIFEST_DIR")?;
        fs::write(format!("{}/../feedstock-stats.toml", path), toml_content)?;
        println!("💾 Updated conversion credits saved to feedstock-stats.toml");
    }

    // Generate leaderboard from all credited conversions
    let mut conversion_counts: HashMap<String, u32> = HashMap::new();
    for (_, entry) in &stats.feedstock_states {
        if entry.recipe_type == RecipeType::RecipeV1 {
            if let Some(ref conversion_credit) = entry.conversion_credit {
                for email in conversion_credit.get_emails() {
                    *conversion_counts.entry(email.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    if conversion_counts.is_empty() {
        println!("📊 No conversion credits found yet");
        return Ok(());
    }

    // Sort by conversion count
    let mut leaderboard: Vec<(String, u32)> = conversion_counts.into_iter().collect();
    leaderboard.sort_by(|a, b| b.1.cmp(&a.1));

    // Display leaderboard (usernames are already resolved during collection)
    println!("\n🏆 Recipe v1 Conversion Leaderboard:");
    println!("=====================================");
    for (index, (contributor, count)) in leaderboard.iter().take(limit).enumerate() {
        println!("{}. {} - {} conversions", index + 1, contributor, count);
    }

    let total_credited = stats
        .feedstock_states
        .values()
        .filter(|entry| {
            entry.recipe_type == RecipeType::RecipeV1 && entry.conversion_credit.is_some()
        })
        .count();
    let total_recipe_v1 = stats
        .feedstock_states
        .values()
        .filter(|entry| entry.recipe_type == RecipeType::RecipeV1)
        .count();

    println!("\n📈 Statistics:");
    println!("Total Recipe v1 feedstocks: {}", total_recipe_v1);
    println!("Credited conversions: {}", total_credited);
    println!(
        "Uncredited conversions: {}",
        total_recipe_v1 - total_credited
    );

    // Save updated stats
    let toml_content = toml::to_string_pretty(&stats)?;
    let path = std::env::var("CARGO_MANIFEST_DIR")?;
    fs::write(format!("{}/../feedstock-stats.toml", path), toml_content)?;

    Ok(())
}

async fn analyze_feedstock_conversion_credit(
    feedstock_name: &str,
    _verbose: bool,
) -> Result<Option<(String, String)>> {
    let repo_url = format!("https://github.com/conda-forge/{}.git", feedstock_name);
    let temp_dir = std::env::temp_dir().join(format!("feedstock_analysis_{}", feedstock_name));

    // Clean up any existing temp directory
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    // Clone repository with shallow history
    let clone_result = Command::new("git")
        .args(&[
            "clone",
            "--depth=50", // Usually enough to find recipe.yaml addition
            &repo_url,
            temp_dir.to_str().unwrap(),
        ])
        .output()?;

    if !clone_result.status.success() {
        fs::remove_dir_all(&temp_dir).ok(); // Clean up on failure
        return Err(anyhow::anyhow!(
            "Failed to clone {}: {}",
            repo_url,
            String::from_utf8_lossy(&clone_result.stderr)
        ));
    }

    // Extract commit SHA and author email of commit that added recipe.yaml
    let git_log_result = Command::new("git")
        .current_dir(&temp_dir)
        .args(&[
            "log",
            "--follow",
            "--diff-filter=A",
            "--format=%H %ae",
            "-1",
            "--",
            "recipe/recipe.yaml",
        ])
        .output()?;

    // Clean up temp directory
    fs::remove_dir_all(&temp_dir).ok();

    if !git_log_result.status.success() {
        return Ok(None); // No recipe.yaml found in history
    }

    let output = String::from_utf8_lossy(&git_log_result.stdout);
    let trimmed_output = output.trim();
    if trimmed_output.is_empty() {
        return Ok(None);
    }

    // Parse "commit_sha author_email" format
    let parts: Vec<&str> = trimmed_output.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(None);
    }

    let commit_sha = parts[0].to_string();
    let author_email = parts[1].to_string();

    Ok(Some((commit_sha, author_email)))
}

async fn resolve_github_username_from_commit(
    feedstock_name: &str,
    commit_sha: &str,
    client: &reqwest::Client,
) -> Result<Option<String>> {
    // Check if GitHub token is available
    let github_token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
    if github_token.is_empty() {
        // Skip GraphQL query if no token is available
        return Ok(None);
    }
    let query = r#"
        query($owner: String!, $name: String!, $oid: GitObjectID!) {
            repository(owner: $owner, name: $name) {
                object(oid: $oid) {
                    ... on Commit {
                        author {
                            user {
                                login
                            }
                        }
                    }
                }
            }
        }
    "#;

    let graphql_query = GraphQLQuery {
        query: query.to_string(),
        variables: GraphQLVariables {
            owner: "conda-forge".to_string(),
            name: feedstock_name.to_string(),
            oid: commit_sha.to_string(),
        },
    };

    let response = client
        .post("https://api.github.com/graphql")
        .header("User-Agent", "conda-forge-leaderboard-tool")
        .header("Authorization", &format!("Bearer {}", github_token))
        .json(&graphql_query)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        println!("🔍 GraphQL request failed: {} - {}", status, text);
        return Ok(None);
    }

    let graphql_response: GraphQLResponse = response.json().await.map_err(|e| {
        anyhow::anyhow!("Failed to parse GraphQL response: {}", e)
    })?;

    Ok(graphql_response
        .data
        .and_then(|data| data.repository)
        .and_then(|repo| repo.object)
        .and_then(|commit| commit.author.user)
        .map(|user| user.login))
}

fn resolve_github_username_fallback(email: &str) -> Option<String> {
    // Handle special GitHub email formats
    if email.contains("@users.noreply.github.com") {
        // Extract username from noreply email format: "123456+username@users.noreply.github.com"
        if let Some(at_pos) = email.find('@') {
            let local_part = &email[..at_pos];
            if let Some(plus_pos) = local_part.find('+') {
                let username = &local_part[plus_pos + 1..];
                return Some(username.to_string());
            }
        }
    }
    None
}

>>>>>>> Stashed changes
fn calculate_top_unconverted_feedstocks(
    feedstock_states: &BTreeMap<String, FeedstockEntry>,
    download_counts: &HashMap<String, u64>,
    limit: usize,
) -> Vec<TopFeedstock> {
    let mut unconverted_with_downloads: Vec<TopFeedstock> = feedstock_states
        .iter()
        .filter(|(_, entry)| entry.recipe_type != RecipeType::RecipeV1)
        .filter_map(|(name, entry)| {
            download_counts.get(name).map(|&downloads| TopFeedstock {
                name: name.clone(),
                downloads,
                recipe_type: entry.recipe_type.clone(),
            })
        })
        .collect();
    
    // Sort by downloads in descending order
    unconverted_with_downloads.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    
    // Take top N
    unconverted_with_downloads.into_iter().take(limit).collect()
}
