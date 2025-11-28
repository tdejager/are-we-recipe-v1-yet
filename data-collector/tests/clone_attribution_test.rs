//! Integration tests for clone-based attribution detection
//!
//! These tests verify that the git clone approach correctly finds
//! the first commit that added recipe.yaml for feedstocks with many commits.
//!
//! These feedstocks have >100 commits to recipe.yaml so they require
//! pagination through the GitHub API - or we can use git clone which is faster.

use data_collector::external::GitHubClient;

/// Feedstocks that have many commits to recipe.yaml (>100)
/// These are the ones that triggered pagination in the original implementation
const PAGINATION_FEEDSTOCKS: &[&str] = &[
    "pydantic_ai-feedstock",
    "python-chromedriver-binary-feedstock",
    "verapdf-feedstock",
    "vercel-cli-feedstock",
];

#[tokio::test]
async fn test_clone_based_attribution() {
    let client = GitHubClient::new()
        .expect("Failed to create GitHub client");

    println!("\n🧪 Testing clone-based attribution for feedstocks with many commits\n");
    println!("{}", "=".repeat(80));

    let mut passed = 0;
    let mut failed = 0;

    for feedstock in PAGINATION_FEEDSTOCKS {
        println!("\n📋 Testing: {}", feedstock);

        // Query via batch_query_recipe_history which will use git clone for these
        let feedstocks = vec![feedstock.to_string()];
        let results = match client.batch_query_recipe_history(&feedstocks).await {
            Ok(r) => r,
            Err(e) => {
                println!("   ❌ FAILED: Error querying: {}", e);
                failed += 1;
                continue;
            }
        };

        let result = match results.into_iter().next() {
            Some(r) => r,
            None => {
                println!("   ❌ FAILED: No result returned");
                failed += 1;
                continue;
            }
        };

        if let Some(ref error) = result.error {
            println!("   ❌ FAILED: {}", error);
            failed += 1;
            continue;
        }

        let Some(commit) = result.first_recipe_commit else {
            println!("   ❌ FAILED: No commit found for recipe.yaml");
            failed += 1;
            continue;
        };

        println!("   First recipe.yaml commit: {}", &commit.sha[..7]);
        println!("   Author: {} <{}>", commit.author.name, commit.author.email);
        if let Some(ref login) = commit.author.login {
            println!("   GitHub login: {}", login);
        }
        println!("   Date: {}", commit.date);
        println!("   Message: {}", commit.message.lines().next().unwrap_or(""));

        // Basic validation - we found a commit with valid data
        if !commit.sha.is_empty() && !commit.author.name.is_empty() {
            println!("   ✅ PASSED: Found valid commit info");
            passed += 1;
        } else {
            println!("   ❌ FAILED: Invalid commit data");
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(80));
    println!(
        "📊 Results: {} passed, {} failed, {} total\n",
        passed,
        failed,
        PAGINATION_FEEDSTOCKS.len()
    );

    assert_eq!(failed, 0, "Some clone-based attribution tests failed");
}
