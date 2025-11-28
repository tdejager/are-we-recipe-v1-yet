use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use rattler_conda_types::Version;
use serde::Deserialize;
use std::collections::HashMap;

const GRAPHQL_ENDPOINT: &str = "https://prefix.dev/api/graphql";
const CONCURRENT_REQUESTS: usize = 50;
const PACKAGES_PER_PAGE: u32 = 50;
const TOP_VERSIONS_LIMIT: usize = 10;

// GraphQL response types
#[derive(Deserialize)]
struct GraphQLResponse {
    data: Option<ChannelData>,
}

#[derive(Deserialize)]
struct ChannelData {
    channel: Option<Channel>,
}

#[derive(Deserialize)]
struct Channel {
    packages: PackagePage,
}

#[derive(Deserialize)]
struct PackagePage {
    #[serde(default)]
    page: Vec<Package>,
    pages: Option<u32>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    #[serde(rename = "downloadCounts")]
    download_counts: Vec<DownloadCount>,
}

#[derive(Deserialize)]
struct DownloadCount {
    count: u64,
    version: String,
}

/// Fetch download counts for all conda-forge packages from prefix.dev GraphQL API
pub async fn fetch_download_counts() -> Result<HashMap<String, u64>> {
    let client = reqwest::Client::new();

    // First, fetch to get total page count
    let total_pages = fetch_page_count(&client).await?;
    println!("📊 Found {} pages of packages to fetch", total_pages);

    // Set up progress bar
    let pb = ProgressBar::new(total_pages as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("⬇️  Fetching downloads: [{bar:40.cyan/blue}] {pos}/{len} pages ({eta})")
            .unwrap()
            .progress_chars("█▓░"),
    );

    // Fetch all pages concurrently with limited parallelism
    let results: Vec<Result<Vec<Package>>> = stream::iter(1..=total_pages)
        .map(|page| {
            let client = client.clone();
            async move { fetch_page(&client, page).await }
        })
        .buffer_unordered(CONCURRENT_REQUESTS)
        .inspect(|_| pb.inc(1))
        .collect()
        .await;

    pb.finish_with_message("✅ Download counts fetched!");

    // Process results into HashMap
    let mut download_counts = HashMap::new();

    for result in results {
        match result {
            Ok(packages) => {
                for pkg in packages {
                    let total = aggregate_top_versions(&pkg.download_counts, TOP_VERSIONS_LIMIT);
                    if total > 0 {
                        let feedstock_name = format!("{}-feedstock", pkg.name);
                        download_counts.insert(feedstock_name, total);
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  Warning: Failed to fetch page: {}", e);
            }
        }
    }

    println!(
        "📦 Fetched download counts for {} packages",
        download_counts.len()
    );

    Ok(download_counts)
}

/// Fetch the total number of pages from the API
async fn fetch_page_count(client: &reqwest::Client) -> Result<u32> {
    let query = format!(
        r#"{{
            channel(name: "conda-forge") {{
                packages(limit: {}) {{
                    pages
                }}
            }}
        }}"#,
        PACKAGES_PER_PAGE
    );

    let response: GraphQLResponse = client
        .post(GRAPHQL_ENDPOINT)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await
        .context("Failed to fetch page count")?
        .json()
        .await
        .context("Failed to parse page count response")?;

    response
        .data
        .and_then(|d| d.channel)
        .and_then(|c| c.packages.pages)
        .context("No page count in response")
}

/// Fetch a single page of packages with their download counts
async fn fetch_page(client: &reqwest::Client, page: u32) -> Result<Vec<Package>> {
    let query = format!(
        r#"{{
            channel(name: "conda-forge") {{
                packages(limit: {}, page: {}) {{
                    page {{
                        name
                        downloadCounts(aggregateBy: [PLATFORM, DATE, PYTHON]) {{
                            count
                            version
                        }}
                    }}
                }}
            }}
        }}"#,
        PACKAGES_PER_PAGE, page
    );

    let response: GraphQLResponse = client
        .post(GRAPHQL_ENDPOINT)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await
        .context(format!("Failed to fetch page {}", page))?
        .json()
        .await
        .context(format!("Failed to parse page {} response", page))?;

    Ok(response
        .data
        .and_then(|d| d.channel)
        .map(|c| c.packages.page)
        .unwrap_or_default())
}

/// Aggregate download counts for the top N versions (sorted by version descending)
fn aggregate_top_versions(counts: &[DownloadCount], limit: usize) -> u64 {
    if counts.is_empty() {
        return 0;
    }

    // Group counts by version
    let mut by_version: HashMap<&str, u64> = HashMap::new();
    for c in counts {
        *by_version.entry(&c.version).or_default() += c.count;
    }

    // Sort versions using rattler's Version type (handles conda versioning correctly)
    let mut versions: Vec<_> = by_version.into_iter().collect();
    versions.sort_by(|(a, _), (b, _)| {
        match (a.parse::<Version>(), b.parse::<Version>()) {
            (Ok(va), Ok(vb)) => vb.cmp(&va), // Descending (newest first)
            (Ok(_), Err(_)) => std::cmp::Ordering::Less, // Valid versions come first
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => b.cmp(a), // String fallback, descending
        }
    });

    // Sum top N versions
    versions.iter().take(limit).map(|(_, count)| count).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_top_versions() {
        let counts = vec![
            DownloadCount {
                version: "1.0.0".to_string(),
                count: 100,
            },
            DownloadCount {
                version: "2.0.0".to_string(),
                count: 200,
            },
            DownloadCount {
                version: "1.5.0".to_string(),
                count: 150,
            },
            DownloadCount {
                version: "3.0.0".to_string(),
                count: 300,
            },
        ];

        // Top 2 versions: 3.0.0 (300) + 2.0.0 (200) = 500
        assert_eq!(aggregate_top_versions(&counts, 2), 500);

        // Top 3 versions: 3.0.0 (300) + 2.0.0 (200) + 1.5.0 (150) = 650
        assert_eq!(aggregate_top_versions(&counts, 3), 650);

        // All versions
        assert_eq!(aggregate_top_versions(&counts, 10), 750);
    }

    #[test]
    fn test_aggregate_empty() {
        let counts: Vec<DownloadCount> = vec![];
        assert_eq!(aggregate_top_versions(&counts, 10), 0);
    }
}
