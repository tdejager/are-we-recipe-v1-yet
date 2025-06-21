use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    feedstock_states: HashMap<String, RecipeType>,
    #[serde(default)]
    historical_snapshots: Vec<HistoricalSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HistoricalSnapshot {
    timestamp: String,
    total_feedstocks: u32,
    recipe_v1_count: u32,
    meta_yaml_count: u32,
    unknown_count: u32,
    newly_converted: Vec<String>, // Feedstocks that converted to Recipe v1 in this snapshot
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("🚀 Starting conda-forge feedstock analysis...");

    let stats = match cli.command {
        Some(Commands::Analyze { force_clone }) => {
            collect_stats_from_node_attrs(force_clone, cli.verbose)?
        }
        None => collect_stats_from_node_attrs(false, cli.verbose)?,
    };

    // Write to TOML file
    let toml_content =
        toml::to_string_pretty(&stats).context("Failed to serialize stats to TOML")?;

    let path = std::env::var("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR not set")?;
    fs::write(format!("{}/../feedstock-stats.toml", path), toml_content)
        .context("Failed to write feedstock-stats.toml")?;

    println!("✅ Analysis complete!");
    println!("📊 Total feedstocks: {}", stats.total_feedstocks);
    println!("📝 Recipe v1 (recipe.yaml): {}", stats.recipe_v1_count);
    println!("📄 Legacy (meta.yaml): {}", stats.meta_yaml_count);
    println!("❓ Unknown/Other: {}", stats.unknown_count);
    println!("💾 Results saved to feedstock-stats.toml");

    Ok(())
}

fn load_existing_stats_if_exists() -> Option<FeedstockStats> {
    let stats_file = "../feedstock-stats.toml";
    let content = fs::read_to_string(stats_file).ok()?;
    let stats: FeedstockStats = toml::from_str(&content).ok()?;
    println!(
        "📂 Loaded existing stats: {} total feedstocks",
        stats.total_feedstocks
    );
    Some(stats)
}

fn collect_stats_from_node_attrs(force_reload: bool, verbose: bool) -> Result<FeedstockStats> {
    // Load existing stats for historical comparison
    let existing_stats = load_existing_stats_if_exists();

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

    let mut feedstock_states = HashMap::new();
    let mut processed = 0;

    // Process each JSON file
    for entry in json_files {
        match parse_node_attrs_file(entry.path()) {
            Ok(node_data) => {
                let feedstock_name = format!("{}-feedstock", node_data.feedstock_name);
                let recipe_type = determine_recipe_type_from_node(&node_data);

                feedstock_states.insert(feedstock_name, recipe_type);
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
        .filter(|&t| *t == RecipeType::RecipeV1)
        .count() as u32;
    let meta_yaml_count = feedstock_states
        .values()
        .filter(|&t| *t == RecipeType::MetaYaml)
        .count() as u32;
    let unknown_count = feedstock_states
        .values()
        .filter(|&t| *t == RecipeType::Unknown)
        .count() as u32;
    let total_feedstocks = processed;

    println!(
        "📝 Recipe v1 (rattler-build + schema_version=1): {}",
        recipe_v1_count
    );
    println!("📄 Legacy (conda-build or other): {}", meta_yaml_count);
    println!("❓ Unknown/Other: {}", unknown_count);

    // Track historical changes
    let mut historical_snapshots = existing_stats
        .as_ref()
        .map(|s| s.historical_snapshots.clone())
        .unwrap_or_default();

    // Find newly converted feedstocks
    let newly_converted = if let Some(ref existing) = existing_stats {
        feedstock_states
            .iter()
            .filter(|(name, recipe_type)| {
                **recipe_type == RecipeType::RecipeV1
                    && existing
                        .feedstock_states
                        .get(*name)
                        .map_or(true, |old_type| *old_type != RecipeType::RecipeV1)
            })
            .map(|(name, _)| name.clone())
            .collect()
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

    // Add current snapshot to history
    let current_snapshot = HistoricalSnapshot {
        timestamp: Utc::now().to_rfc3339(),
        total_feedstocks,
        recipe_v1_count,
        meta_yaml_count,
        unknown_count,
        newly_converted,
    };

    historical_snapshots.push(current_snapshot);

    // Keep only last 100 snapshots to prevent file from growing too large
    if historical_snapshots.len() > 100 {
        let start_idx = historical_snapshots.len() - 100;
        historical_snapshots = historical_snapshots.into_iter().skip(start_idx).collect();
    }

    Ok(FeedstockStats {
        total_feedstocks,
        recipe_v1_count,
        meta_yaml_count,
        unknown_count,
        last_updated: Utc::now().to_rfc3339(),
        feedstock_states,
        historical_snapshots,
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
        println!("📂 Using existing sparse checkout repository");

        // Verify that node_attrs directory exists
        let node_attrs_check = repo_path.join("node_attrs");
        if !node_attrs_check.exists() {
            println!("⚠️  node_attrs directory missing, will re-create sparse checkout...");
            fs::remove_dir_all(repo_path).context("Failed to remove incomplete repository")?;
            return ensure_sparse_checkout_repo(true, verbose); // Recursive call to re-create
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
