use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;

use crate::external::{fetch_recipe_maintainers, CommitAuthor, FirstRecipeCommit, GitHubClient, RecipeHistoryResult};
use crate::models::{Attribution, ContributionType, FeedstockEntry, RecipeCommitCache, RecipeType};

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
/// If `refetch_recipe_commits` is true, also clears the commit cache (forces re-fetch from API).
/// The `save_fn` callback is called after the batch query to save intermediate progress.
pub async fn collect_attributions<F>(
    feedstock_states: &mut BTreeMap<String, FeedstockEntry>,
    verbose: bool,
    reattribute: bool,
    refetch_recipe_commits: bool,
    save_fn: F,
) -> Result<u32>
where
    F: Fn(&BTreeMap<String, FeedstockEntry>) -> Result<()>,
{
    // If refetch flag is set, clear the commit cache
    if refetch_recipe_commits {
        println!("🗑️  Clearing recipe commit cache (--refetch-recipe-commits flag set)");
        for entry in feedstock_states.values_mut() {
            entry.recipe_commit_cache = None;
        }
    }

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

    let mut attributed_count = 0u32;

    // Check which feedstocks have cached commit info (from previous interrupted run)
    let (cached, needs_fetch): (Vec<_>, Vec<_>) = needs_attribution
        .iter()
        .partition(|name| {
            feedstock_states
                .get(*name)
                .and_then(|e| e.recipe_commit_cache.as_ref())
                .is_some()
        });

    // Build results from cache + fresh fetch
    let batch_results: Vec<RecipeHistoryResult> = if !cached.is_empty() {
        println!(
            "📦 Found {} feedstocks with cached commit info, {} need fetching",
            cached.len(),
            needs_fetch.len()
        );

        // Convert cached entries to RecipeHistoryResult
        let mut results: Vec<RecipeHistoryResult> = cached
            .iter()
            .filter_map(|name| {
                let entry = feedstock_states.get(*name)?;
                let cache = entry.recipe_commit_cache.as_ref()?;
                Some(RecipeHistoryResult {
                    feedstock: (*name).clone(),
                    first_recipe_commit: Some(FirstRecipeCommit {
                        sha: cache.sha.clone(),
                        message: cache.message.clone(),
                        date: cache.date.clone(),
                        author: CommitAuthor {
                            login: cache.author_login.clone(),
                            name: cache.author_name.clone(),
                            email: cache.author_email.clone(),
                        },
                    }),
                    error: None,
                })
            })
            .collect();

        // Fetch remaining
        if !needs_fetch.is_empty() {
            let fetch_names: Vec<String> = needs_fetch.into_iter().cloned().collect();
            let fetched = github_client
                .batch_query_recipe_history(&fetch_names)
                .await?;
            results.extend(fetched);
        }

        results
    } else {
        // No cache, fetch all
        github_client
            .batch_query_recipe_history(&needs_attribution)
            .await?
    };

    // Save commit info to cache for resume capability
    for result in &batch_results {
        if let Some(commit) = &result.first_recipe_commit {
            if let Some(entry) = feedstock_states.get_mut(&result.feedstock) {
                entry.recipe_commit_cache = Some(RecipeCommitCache {
                    sha: commit.sha.clone(),
                    message: commit.message.clone(),
                    date: commit.date.clone(),
                    author_login: commit.author.login.clone(),
                    author_name: commit.author.name.clone(),
                    author_email: commit.author.email.clone(),
                });
            }
        }
    }

    // Save checkpoint after batch query completes (step 1-2 done)
    println!("💾 Saving checkpoint (batch query complete)...");
    save_fn(feedstock_states)?;

    // Determine new feedstocks by checking if the first recipe.yaml commit
    // is an "Initial feedstock commit" - no cloning needed!
    let new_feedstock_set: std::collections::HashSet<String> = batch_results
        .iter()
        .filter(|r| {
            r.first_recipe_commit
                .as_ref()
                .map(|c| is_initial_feedstock_commit(&c.message))
                .unwrap_or(false)
        })
        .map(|r| r.feedstock.clone())
        .collect();

    println!(
        "🔍 Found {} new feedstocks, {} conversions",
        new_feedstock_set.len(),
        needs_attribution.len() - new_feedstock_set.len()
    );

    // Set up progress bar for attribution processing
    let pb = ProgressBar::new(batch_results.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap(),
    );

    for result in batch_results {
        pb.inc(1);

        let is_new_feedstock = new_feedstock_set.contains(&result.feedstock);

        if let Some(attribution) =
            process_history_result(&result, &github_client, verbose, is_new_feedstock).await
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
    is_new_feedstock: bool,
) -> Option<Attribution> {
    let commit = result.first_recipe_commit.as_ref()?;

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

/// Check if a commit message indicates an initial feedstock commit
/// This is used to identify new feedstocks vs conversions without cloning
fn is_initial_feedstock_commit(message: &str) -> bool {
    let msg_lower = message.to_lowercase();
    msg_lower.contains("initial feedstock commit")
        || msg_lower.starts_with("initial commit")
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
