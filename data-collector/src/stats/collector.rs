use anyhow::Result;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::CF_GRAPH_LOCAL_PATH;
use crate::external::fetch_download_counts;
use crate::git::ensure_sparse_checkout_repo;
use crate::models::{FeedstockEntry, FeedstockStats, RecipeType};
use crate::stats::{
    calculate_top_unconverted_feedstocks, determine_recipe_type_from_node, parse_node_attrs_file,
};

pub fn load_existing_stats_if_exists() -> Option<FeedstockStats> {
    let path = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let stats_file = format!("{}/../feedstock-stats.toml", path);
    load_existing_stats(&stats_file).ok()
}

/// Load existing stats from a specific path
pub fn load_existing_stats(stats_path: &str) -> Result<FeedstockStats> {
    println!("🔍 Loading stats from: {}", stats_path);
    let content = fs::read_to_string(stats_path)?;
    let stats: FeedstockStats = toml::from_str(&content)?;
    println!(
        "📂 Loaded existing stats: {} total feedstocks, {} feedstock_states entries",
        stats.total_feedstocks,
        stats.feedstock_states.len()
    );
    Ok(stats)
}

/// Collect feesdstock statistics from node attributes files.
/// Which are present in the `node_attrs` directory of the sparse checkout repository.
pub async fn collect_stats_from_node_attrs(
    force_reload: bool,
    verbose: bool,
) -> Result<FeedstockStats> {
    // Load existing stats for historical comparison
    let existing_stats = load_existing_stats_if_exists();

    // Fetch download counts
    println!("📥 Fetching download counts from prefix.dev...");
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
            entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "json")
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
                            && recipe_type == RecipeType::RecipeV1
                        {
                            if verbose {
                                println!(
                                    "🔄 CONVERTED: {} from {:?} to {:?}",
                                    feedstock_name, existing_entry.recipe_type, recipe_type
                                );
                            }
                            current_time.clone() // Converted to RecipeV1, update timestamp
                        } else {
                            if verbose && processed < 5 {
                                println!(
                                    "📌 KEEPING: {} - {:?} (old: {}, keeping: {})",
                                    feedstock_name,
                                    recipe_type,
                                    current_time,
                                    existing_entry.last_changed
                                );
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

                // Preserve existing attribution if present
                let attribution = if let Some(ref existing) = existing_stats {
                    existing
                        .feedstock_states
                        .get(&feedstock_name)
                        .and_then(|e| e.attribution.clone())
                } else {
                    None
                };

                // Look up download count for this feedstock
                let downloads = download_counts.get(&feedstock_name).copied();

                feedstock_states.insert(
                    feedstock_name,
                    FeedstockEntry {
                        recipe_type,
                        last_changed,
                        attribution,
                        downloads,
                        recipe_commit_cache: None,
                    },
                );
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
                        .is_none_or(|old_entry| old_entry.recipe_type != RecipeType::RecipeV1)
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
    let top_unconverted =
        calculate_top_unconverted_feedstocks(&feedstock_states, &download_counts, 50);
    println!(
        "🏆 Found {} top unconverted feedstocks by downloads",
        top_unconverted.len()
    );

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
