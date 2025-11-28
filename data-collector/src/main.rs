use anyhow::{Context, Result};
use clap::Parser;
use std::fs;

use data_collector::git::cleanup_sparse_checkout_repo;
use data_collector::models::*;
use data_collector::stats::{collect_attributions, collect_stats_from_node_attrs, load_existing_stats};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // --reattribute-only mode: skip analysis/downloads, just reload and re-attribute
    let mut stats = if cli.reattribute_only {
        println!("🔄 Running attribution-only mode...");
        let path = std::env::var("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR not set")?;
        let stats_path = format!("{}/../feedstock-stats.toml", path);
        load_existing_stats(&stats_path).context("Failed to load existing stats - run full analysis first")?
    } else {
        println!("🚀 Starting conda-forge feedstock analysis...");

        match cli.command {
            Some(Commands::Analyze { force_clone }) => {
                collect_stats_from_node_attrs(force_clone, cli.verbose).await?
            }
            None => collect_stats_from_node_attrs(false, cli.verbose).await?,
        }
    };

    // Collect attribution data for Recipe v1 feedstocks
    println!("\n🏆 Collecting contributor attribution...");
    let reattribute = cli.reattribute || cli.reattribute_only;

    // Create save function for checkpointing
    let stats_path = {
        let path = std::env::var("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR not set")?;
        format!("{}/../feedstock-stats.toml", path)
    };
    let save_checkpoint = |feedstock_states: &std::collections::BTreeMap<String, FeedstockEntry>| {
        let checkpoint_stats = FeedstockStats {
            total_feedstocks: stats.total_feedstocks,
            recipe_v1_count: stats.recipe_v1_count,
            meta_yaml_count: stats.meta_yaml_count,
            unknown_count: stats.unknown_count,
            last_updated: stats.last_updated.clone(),
            feedstock_states: feedstock_states.clone(),
            top_unconverted_by_downloads: stats.top_unconverted_by_downloads.clone(),
        };
        let toml_content = toml::to_string_pretty(&checkpoint_stats)
            .context("Failed to serialize stats to TOML")?;
        fs::write(&stats_path, toml_content).context("Failed to write checkpoint")?;
        Ok(())
    };

    let attributed = collect_attributions(
        &mut stats.feedstock_states,
        cli.verbose,
        reattribute,
        cli.refetch_recipe_commits,
        save_checkpoint,
    )
    .await?;
    if attributed > 0 {
        println!("📝 Attributed {} feedstocks", attributed);
    }

    // Write final stats to TOML file
    let toml_content =
        toml::to_string_pretty(&stats).context("Failed to serialize stats to TOML")?;
    fs::write(&stats_path, toml_content).context("Failed to write feedstock-stats.toml")?;

    // Clean up sparse checkout repository (only if we did full analysis)
    if !cli.reattribute_only {
        cleanup_sparse_checkout_repo(cli.verbose)?;
    }

    println!("\n✅ Analysis complete!");
    println!("📊 Total feedstocks: {}", stats.total_feedstocks);
    println!("📝 Recipe v1 (recipe.yaml): {}", stats.recipe_v1_count);
    println!("📄 Legacy (meta.yaml): {}", stats.meta_yaml_count);
    println!("❓ Unknown/Other: {}", stats.unknown_count);
    println!("💾 Results saved to feedstock-stats.toml");

    Ok(())
}
