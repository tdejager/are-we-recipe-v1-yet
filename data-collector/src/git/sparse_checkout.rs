use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::{CF_GRAPH_LOCAL_PATH, CF_GRAPH_REPO_URL};

pub fn ensure_sparse_checkout_repo(force_reload: bool, verbose: bool) -> Result<()> {
    let repo_path = Path::new(CF_GRAPH_LOCAL_PATH);

    if force_reload && repo_path.exists() {
        println!("🗑️  Removing existing repository for fresh sparse checkout...");
        fs::remove_dir_all(repo_path).context("Failed to remove existing repository")?;
    }

    if !repo_path.exists() {
        println!("📥 Creating sparse checkout of conda-forge-bot-data-node_attrs repository...");
        println!("🎯 Only downloading sharded node_attrs directories (much faster than full clone)");

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
            .args(["remote", "add", "origin", CF_GRAPH_REPO_URL])
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
            .args(["config", "core.sparseCheckout", "true"])
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
        // Node attrs are sharded at the repo root as `<hex>/<hex>/<hex>/<hex>/<hex>/<name>.json`
        fs::write(&sparse_checkout_path, "/[0-9a-f]/\n")
            .context("Failed to write sparse-checkout file")?;

        if verbose {
            println!("✅ Sparse checkout pattern set to /[0-9a-f]/");
        }

        // Pull with depth=1
        let pull_result = Command::new("git")
            .current_dir(repo_path)
            .args(["pull", "origin", "main", "--depth=1"])
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
                .args(["-la"])
                .output()
                .context("Failed to list directory contents")?;

            if ls_result.status.success() {
                println!("{}", String::from_utf8_lossy(&ls_result.stdout));
            }
        }
    } else {
        // Check if existing sparse checkout is valid
        if repo_path.join(".git").exists() && repo_path.join("0").exists() {
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

pub fn cleanup_sparse_checkout_repo(verbose: bool) -> Result<()> {
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
