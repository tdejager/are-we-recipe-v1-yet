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
///
/// If `reattribute` is true, clears existing attributions and re-calculates all.
pub async fn collect_attributions(
    feedstock_states: &mut BTreeMap<String, FeedstockEntry>,
    verbose: bool,
    reattribute: bool,
) -> Result<u32> {
    // If reattribute flag is set, clear all existing attributions first
    if reattribute {
        println!("🔄 Re-calculating all attributions (--reattribute flag set)");
        for entry in feedstock_states.values_mut() {
            if entry.recipe_type == RecipeType::RecipeV1 {
                entry.attribution = None;
            }
        }
    }

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

        if let Some(attribution) =
            process_history_result(&result, &github_client, verbose).await
        {
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
///
/// New attribution rules:
/// 1. New Feedstock: recipe.yaml exists in the very first commit of the repo
///    -> Credit goes to maintainers from recipe.yaml
/// 2. Conversion: recipe.yaml was added in a later commit
///    -> Look up the PR, credit the PR author (or commit author who added recipe.yaml if bot PR)
async fn process_history_result(
    result: &RecipeHistoryResult,
    github_client: &GitHubClient,
    verbose: bool,
) -> Option<Attribution> {
    let commit = result.first_recipe_commit.as_ref()?;

    // Rule 1: Check if this is a new feedstock (recipe.yaml in first commit)
    let is_new_feedstock = github_client
        .has_recipe_yaml_in_first_commit(&result.feedstock)
        .await
        .unwrap_or(false);

    if is_new_feedstock {
        // New feedstock - credit the maintainers from recipe.yaml
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

        return Some(Attribution {
            contribution_type: ContributionType::NewFeedstock,
            contributors,
            date: commit.date.clone(),
            commit_sha: Some(commit.sha.clone()),
        });
    }

    // Rule 2: This is a conversion - find who did it
    let contributor = find_conversion_contributor(result, github_client, commit, verbose).await;

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

/// Find who actually did the conversion by looking at PRs and commits
async fn find_conversion_contributor(
    result: &RecipeHistoryResult,
    github_client: &GitHubClient,
    commit: &crate::external::FirstRecipeCommit,
    verbose: bool,
) -> String {
    // Try to find the PR for this commit
    let pr_info = github_client
        .get_pr_for_commit(&result.feedstock, &commit.sha)
        .await
        .ok()
        .flatten();

    match pr_info {
        Some(pr) => {
            // Check if PR author is a bot
            if is_bot_username(&pr.author) {
                // Bot opened PR - find who actually added recipe.yaml
                if verbose {
                    println!(
                        "    PR #{} opened by bot {}, looking for human contributor...",
                        pr.number, pr.author
                    );
                }

                // Get PR commits and find who added recipe.yaml
                if let Ok(commits) = github_client
                    .get_pr_commits(&result.feedstock, pr.number)
                    .await
                {
                    for pr_commit in &commits {
                        // Check if this commit added recipe.yaml
                        if let Ok(true) = github_client
                            .commit_has_recipe_yaml(&result.feedstock, &pr_commit.sha)
                            .await
                        {
                            // Found the commit that added recipe.yaml
                            if !is_bot_username(&pr_commit.author) {
                                if verbose {
                                    println!(
                                        "    Found human contributor: {} (commit {})",
                                        pr_commit.author,
                                        &pr_commit.sha[..7]
                                    );
                                }
                                return pr_commit.author.clone();
                            }
                        }
                    }
                }

                // Fallback: couldn't find human contributor in PR commits
                // Use commit author as fallback
                if verbose {
                    println!("    Could not find human contributor in PR, using commit author");
                }
                commit
                    .author
                    .login
                    .clone()
                    .unwrap_or_else(|| commit.author.name.clone())
            } else {
                // Human opened PR - credit them
                pr.author
            }
        }
        None => {
            // No PR found - direct push, credit commit author
            if verbose {
                println!("    No PR found, using commit author");
            }
            commit
                .author
                .login
                .clone()
                .unwrap_or_else(|| commit.author.name.clone())
        }
    }
}

/// Check if a username looks like a bot
fn is_bot_username(username: &str) -> bool {
    let username_lower = username.to_lowercase();
    BOT_PATTERNS
        .iter()
        .any(|pattern| username_lower.contains(pattern))
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
