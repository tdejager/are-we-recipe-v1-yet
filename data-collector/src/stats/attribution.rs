use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;

use crate::external::{fetch_recipe_maintainers, CommitAuthor, GitHubClient, RecipeHistoryResult};
use crate::models::{Attribution, ContributionType, FeedstockEntry, RecipeType};

/// Known bot patterns for detecting automated commits
const BOT_PATTERNS: &[&str] = &[
    "conda-forge-admin",
    "regro-cf-autotick-bot",
    "conda-forge-linter",
    "[bot]",
    "github-actions",
    "conda-forge-daemon",
    "conda-forge-coordinator",
    "conda-forge-webservices",
    "conda-forge-status",
];

/// Determine if a commit author is a bot
pub fn is_bot_author(author: &CommitAuthor) -> bool {
    let login_lower = author
        .login
        .as_ref()
        .map(|l| l.to_lowercase())
        .unwrap_or_default();
    let name_lower = author.name.to_lowercase();
    let email_lower = author.email.to_lowercase();

    BOT_PATTERNS.iter().any(|pattern| {
        login_lower.contains(pattern)
            || name_lower.contains(pattern)
            || email_lower.contains(pattern)
    })
}

/// Collect attribution data for Recipe v1 feedstocks that don't have it yet
pub async fn collect_attributions(
    feedstock_states: &mut BTreeMap<String, FeedstockEntry>,
    verbose: bool,
) -> Result<u32> {
    // Find feedstocks that need attribution
    let needs_attribution: Vec<String> = feedstock_states
        .iter()
        .filter(|(_, entry)| {
            entry.recipe_type == RecipeType::RecipeV1 && entry.attribution.is_none()
        })
        .map(|(name, _)| name.clone())
        .collect();

    if needs_attribution.is_empty() {
        println!("✅ All Recipe v1 feedstocks already have attribution");
        return Ok(0);
    }

    println!(
        "🔍 Found {} Recipe v1 feedstocks needing attribution",
        needs_attribution.len()
    );

    // Try to create GitHub client
    let github_client = match GitHubClient::new() {
        Ok(client) => client,
        Err(e) => {
            println!("⚠️  GitHub client not available: {}", e);
            println!("   Skipping attribution collection. Set GITHUB_TOKEN or install gh CLI.");
            return Ok(0);
        }
    };

    // Check rate limit
    match github_client.check_rate_limit().await {
        Ok(info) => {
            println!(
                "📊 GitHub API rate limit: {}/{} (resets at {})",
                info.remaining, info.limit, info.reset_at
            );
            if info.remaining < 100 {
                println!("⚠️  Low rate limit. Consider waiting before running attribution.");
            }
        }
        Err(e) => {
            println!("⚠️  Could not check rate limit: {}", e);
        }
    }

    // Set up progress bar
    let pb = ProgressBar::new(needs_attribution.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap(),
    );

    let mut attributed_count = 0u32;

    // Process in batches
    let batch_results = github_client
        .batch_query_recipe_history(&needs_attribution)
        .await?;

    for result in batch_results {
        pb.inc(1);

        if let Some(attribution) = process_history_result(&result, verbose).await {
            if let Some(entry) = feedstock_states.get_mut(&result.feedstock) {
                entry.attribution = Some(attribution);
                attributed_count += 1;
            }
        }
    }

    pb.finish_with_message("Attribution collection complete!");

    println!("✅ Attributed {} feedstocks", attributed_count);

    Ok(attributed_count)
}

/// Process a single history result and determine attribution
async fn process_history_result(
    result: &RecipeHistoryResult,
    verbose: bool,
) -> Option<Attribution> {
    let commit = result.first_recipe_commit.as_ref()?;

    let is_bot = is_bot_author(&commit.author);

    if is_bot {
        // Bot created the commit -> New feedstock (staged-recipes merge)
        // Credit goes to maintainers from recipe.yaml
        let maintainers = fetch_recipe_maintainers(&result.feedstock)
            .await
            .unwrap_or_default();

        let contributors = if maintainers.is_empty() {
            if verbose {
                println!(
                    "  ⚠️  {}: No maintainers found, using 'unknown'",
                    result.feedstock
                );
            }
            vec!["unknown".to_string()]
        } else {
            maintainers
        };

        if verbose {
            println!(
                "  🆕 {}: New feedstock by {:?}",
                result.feedstock, contributors
            );
        }

        Some(Attribution {
            contribution_type: ContributionType::NewFeedstock,
            contributors,
            date: commit.date.clone(),
            commit_sha: Some(commit.sha.clone()),
        })
    } else {
        // Human created the commit -> Conversion
        // Credit goes to commit author
        let contributor = commit
            .author
            .login
            .clone()
            .unwrap_or_else(|| commit.author.name.clone());

        if verbose {
            println!("  🔄 {}: Conversion by {}", result.feedstock, contributor);
        }

        Some(Attribution {
            contribution_type: ContributionType::Conversion,
            contributors: vec![contributor],
            date: commit.date.clone(),
            commit_sha: Some(commit.sha.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bot_author_detects_bots() {
        let bot_author = CommitAuthor {
            login: Some("conda-forge-admin".to_string()),
            name: "Conda Forge Admin".to_string(),
            email: "conda-forge-admin@example.com".to_string(),
        };
        assert!(is_bot_author(&bot_author));

        let bot_author2 = CommitAuthor {
            login: Some("github-actions[bot]".to_string()),
            name: "github-actions[bot]".to_string(),
            email: "41898282+github-actions[bot]@users.noreply.github.com".to_string(),
        };
        assert!(is_bot_author(&bot_author2));
    }

    #[test]
    fn test_is_bot_author_allows_humans() {
        let human_author = CommitAuthor {
            login: Some("johndoe".to_string()),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };
        assert!(!is_bot_author(&human_author));
    }
}
