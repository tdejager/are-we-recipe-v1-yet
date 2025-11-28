//! Integration tests for attribution detection
//!
//! These tests verify that the attribution logic correctly identifies
//! who converted a feedstock to Recipe v1 and properly classifies
//! contributions as either Conversions or New Feedstocks.
//!
//! New attribution rules:
//! 1. New Feedstock: recipe.yaml exists in the very first commit of the repo
//! 2. Conversion: recipe.yaml was added later, credit PR author (or human committer if bot PR)

use data_collector::external::{fetch_recipe_maintainers, GitHubClient};

/// Expected attribution for a feedstock
struct ExpectedAttribution {
    /// The feedstock name (with -feedstock suffix)
    feedstock: &'static str,
    /// Expected contributor (GitHub login)
    expected_contributor: &'static str,
    /// Whether this should be a conversion (true) or new feedstock (false)
    is_conversion: bool,
    /// Description of why this test case is important
    description: &'static str,
}

/// Test cases for attribution detection
const TEST_CASES: &[ExpectedAttribution] = &[
    // === CONVERSIONS (recipe.yaml added to existing feedstock) ===
    //
    // Case 1: Human opened PR to convert
    ExpectedAttribution {
        feedstock: "urdfdom-feedstock",
        expected_contributor: "traversaro",
        is_conversion: true,
        description: "Human opened PR #34 to convert - credit goes to PR author",
    },
    // Case 2: Human directly committed (multiple commits, not squashed)
    ExpectedAttribution {
        feedstock: "libqdldl-feedstock",
        expected_contributor: "sebp",
        is_conversion: true,
        description: "Human directly committed conversion - credit goes to commit author",
    },
    // Case 3: Bot opened PR, human added the conversion commit
    ExpectedAttribution {
        feedstock: "libraw-feedstock",
        expected_contributor: "wolfv",
        is_conversion: true,
        description: "Bot opened PR #19, but wolfv added the recipe.yaml conversion",
    },
    //
    // === NEW FEEDSTOCKS (recipe.yaml from first commit) ===
    //
    // Case 4: New feedstock created with recipe.yaml from the start
    ExpectedAttribution {
        feedstock: "box2d-feedstock",
        expected_contributor: "tdejager", // First maintainer in recipe.yaml
        is_conversion: false,
        description: "New feedstock created with recipe.yaml - credit goes to maintainers",
    },
];

#[tokio::test]
async fn test_attribution_detection() {
    let client = GitHubClient::new()
        .expect("Failed to create GitHub client - ensure gh CLI is installed or GITHUB_TOKEN is set");

    let mut passed = 0;
    let mut failed = 0;

    println!("\n🧪 Running Attribution Detection Tests\n");
    println!("{}", "=".repeat(80));

    for test in TEST_CASES {
        println!("\n📋 Test: {}", test.feedstock);
        println!("   Description: {}", test.description);
        println!(
            "   Expected: {} by {}",
            if test.is_conversion {
                "Conversion"
            } else {
                "New Feedstock"
            },
            test.expected_contributor
        );

        // Step 1: Check if this is a new feedstock (recipe.yaml in first commit)
        let is_new_feedstock = client
            .has_recipe_yaml_in_first_commit(test.feedstock)
            .await
            .unwrap_or(false);

        println!("   Has recipe.yaml in first commit: {}", is_new_feedstock);

        if is_new_feedstock {
            // New feedstock - verify classification and check maintainers
            let classification_correct = !test.is_conversion;

            if !classification_correct {
                println!("   ❌ FAILED: Expected Conversion but got New Feedstock");
                failed += 1;
                continue;
            }

            // Fetch maintainers from recipe.yaml
            let maintainers = fetch_recipe_maintainers(test.feedstock)
                .await
                .unwrap_or_default();
            println!("   Maintainers: {:?}", maintainers);

            let contributor_in_maintainers = maintainers.contains(&test.expected_contributor.to_string());

            if contributor_in_maintainers {
                println!("   ✅ PASSED: Correctly identified as New Feedstock, {} is a maintainer", test.expected_contributor);
                passed += 1;
            } else {
                println!(
                    "   ❌ FAILED: Expected {} to be in maintainers {:?}",
                    test.expected_contributor, maintainers
                );
                failed += 1;
            }
            continue;
        }

        // Step 2: This is a conversion - find the commit that added recipe.yaml
        let feedstocks = vec![test.feedstock.to_string()];
        let results = client
            .batch_query_recipe_history(&feedstocks)
            .await
            .expect("Failed to query GitHub");

        let result = results.into_iter().next().expect("No result returned");

        let Some(commit) = result.first_recipe_commit else {
            println!("   ❌ FAILED: No commit found for recipe.yaml");
            failed += 1;
            continue;
        };

        println!("   First recipe.yaml commit: {}", &commit.sha[..7]);

        // Step 3: Look up the PR for this commit
        let pr_info = client
            .get_pr_for_commit(test.feedstock, &commit.sha)
            .await
            .ok()
            .flatten();

        let contributor = match &pr_info {
            Some(pr) => {
                println!("   PR #{} by {}", pr.number, pr.author);

                // Check if PR author is a bot
                if is_bot_username(&pr.author) {
                    println!("   PR opened by bot, searching for human contributor...");

                    // Get PR commits and find who added recipe.yaml
                    let pr_commits = client
                        .get_pr_commits(test.feedstock, pr.number)
                        .await
                        .unwrap_or_default();

                    let mut found_contributor = None;
                    for pr_commit in &pr_commits {
                        if let Ok(true) = client
                            .commit_has_recipe_yaml(test.feedstock, &pr_commit.sha)
                            .await
                        {
                            if !is_bot_username(&pr_commit.author) {
                                found_contributor = Some(pr_commit.author.clone());
                                println!(
                                    "   Found human contributor: {} (commit {})",
                                    pr_commit.author,
                                    &pr_commit.sha[..7]
                                );
                                break;
                            }
                        }
                    }

                    found_contributor.unwrap_or_else(|| {
                        commit
                            .author
                            .login
                            .clone()
                            .unwrap_or_else(|| commit.author.name.clone())
                    })
                } else {
                    // Human PR author
                    pr.author.clone()
                }
            }
            None => {
                println!("   No PR found, using commit author");
                commit
                    .author
                    .login
                    .clone()
                    .unwrap_or_else(|| commit.author.name.clone())
            }
        };

        println!("   Detected contributor: {}", contributor);

        // Verify results
        let classification_correct = test.is_conversion; // We already know it's not in first commit
        let contributor_correct = contributor == test.expected_contributor;

        if classification_correct && contributor_correct {
            println!("   ✅ PASSED");
            passed += 1;
        } else {
            println!("   ❌ FAILED");
            if !classification_correct {
                println!("      Expected Conversion but got New Feedstock");
            }
            if !contributor_correct {
                println!(
                    "      Expected contributor: {}, got: {}",
                    test.expected_contributor, contributor
                );
            }
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(80));
    println!(
        "📊 Results: {} passed, {} failed, {} total",
        passed,
        failed,
        TEST_CASES.len()
    );
    println!();

    assert_eq!(failed, 0, "Some attribution tests failed");
}

/// Check if a username looks like a bot
fn is_bot_username(username: &str) -> bool {
    let bot_patterns = [
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

    let username_lower = username.to_lowercase();
    bot_patterns
        .iter()
        .any(|pattern| username_lower.contains(pattern))
}
