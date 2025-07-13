use anyhow::{Context, Result};
use clap::Parser;
use std::fs;

use data_collector::models::*;
use data_collector::git::cleanup_sparse_checkout_repo;
use data_collector::stats::collect_stats_from_node_attrs;

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