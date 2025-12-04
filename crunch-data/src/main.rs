use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();

    let input_path = workspace_root.join("feedstock-stats.toml");
    let output_path = workspace_root.join("web/src/stats.toml");

    if let Ok(content) = fs::read_to_string(&input_path) {
        if let Ok(toml_data) = toml::from_str::<toml::Table>(&content) {
            let mut summary = toml::Table::new();

            // Extract only the summary fields we need
            if let Some(total) = toml_data.get("total_feedstocks") {
                summary.insert("total_feedstocks".to_string(), total.clone());
            }
            if let Some(v1_count) = toml_data.get("recipe_v1_count") {
                summary.insert("recipe_v1_count".to_string(), v1_count.clone());
            }
            if let Some(meta_count) = toml_data.get("meta_yaml_count") {
                summary.insert("meta_yaml_count".to_string(), meta_count.clone());
            }
            if let Some(unknown) = toml_data.get("unknown_count") {
                summary.insert("unknown_count".to_string(), unknown.clone());
            }
            if let Some(updated) = toml_data.get("last_updated") {
                summary.insert("last_updated".to_string(), updated.clone());
            }

            // Process feedstock states for recent updates and leaderboard
            if let Some(feedstocks) = toml_data.get("feedstock_states") {
                if let Some(feedstocks_table) = feedstocks.as_table() {
                    // Generate recently updated feedstocks
                    let recent_table = extract_recently_updated(feedstocks_table);
                    summary.insert(
                        "recently_updated".to_string(),
                        toml::Value::Table(recent_table),
                    );

                    // Generate leaderboard from attributions
                    let top_contributors = extract_top_contributors(feedstocks_table);
                    summary.insert(
                        "top_contributors".to_string(),
                        toml::Value::Array(top_contributors),
                    );
                }
            }

            // Include top unconverted feedstocks by downloads
            if let Some(top_unconverted) = toml_data.get("top_unconverted_by_downloads") {
                summary.insert(
                    "top_unconverted_by_downloads".to_string(),
                    top_unconverted.clone(),
                );
            }

            // Write the complete summary
            let summary_toml = toml::to_string(&summary).unwrap();
            fs::write(&output_path, summary_toml).expect("Failed to write summary");
            println!(
                "✅ Crunched feedstock stats written to {}",
                output_path.display()
            );
        }
    }
}

/// Extract the 10 most recently updated Recipe v1 feedstocks with attribution
fn extract_recently_updated(feedstocks_table: &toml::Table) -> toml::Table {
    let mut recent_feedstocks: Vec<_> = feedstocks_table
        .iter()
        .filter_map(|(name, state)| {
            // Only include recipe_v1 feedstocks
            if state
                .get("recipe_type")
                .and_then(|recipe_type| recipe_type.as_str().map(|s| s == "recipe_v1"))
                .unwrap_or(false)
            {
                state.get("last_changed").and_then(|date| {
                    let date_str = date.as_str()?.to_string();
                    // Extract contributors from attribution if available
                    let contributors: Vec<String> = state
                        .get("attribution")
                        .and_then(|attr| attr.get("contributors"))
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    Some((name.clone(), date_str, contributors))
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by last updated date (most recent first)
    recent_feedstocks.sort_by(|(_, a, _), (_, b, _)| b.cmp(a));

    // Take the 10 most recent
    recent_feedstocks.truncate(10);

    // Create a new table for the recent feedstocks
    let mut recent_table = toml::Table::new();
    for (name, date, contributors) in recent_feedstocks {
        let mut entry = toml::Table::new();
        entry.insert("date".to_string(), toml::Value::String(date));
        entry.insert(
            "contributors".to_string(),
            toml::Value::Array(contributors.into_iter().map(toml::Value::String).collect()),
        );
        recent_table.insert(name, toml::Value::Table(entry));
    }

    recent_table
}

/// A single feedstock contribution by a contributor
#[derive(Clone)]
struct FeedstockContribution {
    name: String,
    contribution_type: String,
    downloads: u64,
    date: String,
}

/// Aggregated stats for a single contributor
struct ContributorData {
    conversions: u32,
    new_feedstocks: u32,
    total_downloads: u64,
    feedstocks: Vec<FeedstockContribution>,
}

/// Weekly activity buckets: (conversions, new_feedstocks) for each of the last 20 weeks
/// Index 0 = most recent week, index 19 = oldest week
fn compute_weekly_activity(feedstocks: &[FeedstockContribution]) -> Vec<(u32, u32)> {
    let now = Utc::now();
    let mut weekly: Vec<(u32, u32)> = vec![(0, 0); 20];

    for f in feedstocks {
        if f.date.is_empty() {
            continue;
        }

        // Parse the ISO date
        if let Ok(date) = DateTime::parse_from_rfc3339(&f.date) {
            let date_utc = date.with_timezone(&Utc);
            let days_ago = (now - date_utc).num_days();

            if days_ago >= 0 {
                let weeks_ago = (days_ago / 7) as usize;
                if weeks_ago < 20 {
                    match f.contribution_type.as_str() {
                        "conversion" => weekly[weeks_ago].0 += 1,
                        "new_feedstock" => weekly[weeks_ago].1 += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    weekly
}

/// Extract top contributors from attribution data with enriched statistics
fn extract_top_contributors(feedstocks_table: &toml::Table) -> Vec<toml::Value> {
    // Aggregate contributions by contributor
    let mut contributor_stats: HashMap<String, ContributorData> = HashMap::new();

    for (name, state) in feedstocks_table.iter() {
        if let Some(attribution) = state.get("attribution").and_then(|a| a.as_table()) {
            let contribution_type = attribution
                .get("contribution_type")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let date = attribution
                .get("date")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let downloads = state
                .get("downloads")
                .and_then(|d| d.as_integer())
                .map(|d| d as u64)
                .unwrap_or(0);

            let contributors = attribution
                .get("contributors")
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for contributor in contributors {
                let entry = contributor_stats.entry(contributor).or_insert(ContributorData {
                    conversions: 0,
                    new_feedstocks: 0,
                    total_downloads: 0,
                    feedstocks: Vec::new(),
                });

                match contribution_type.as_str() {
                    "conversion" => entry.conversions += 1,
                    "new_feedstock" => entry.new_feedstocks += 1,
                    _ => {}
                }

                entry.total_downloads += downloads;
                entry.feedstocks.push(FeedstockContribution {
                    name: name.clone(),
                    contribution_type: contribution_type.clone(),
                    downloads,
                    date: date.clone(),
                });
            }
        }
    }

    // Sort by total contributions (descending)
    let mut sorted: Vec<_> = contributor_stats.into_iter().collect();

    sorted.sort_by(|(_, a), (_, b)| {
        let total_a = a.conversions + a.new_feedstocks;
        let total_b = b.conversions + b.new_feedstocks;
        total_b.cmp(&total_a)
    });

    // Take top 50 and convert to TOML with enriched data
    sorted
        .into_iter()
        .take(50)
        .map(|(name, data)| {
            let mut entry = toml::Table::new();
            entry.insert("name".to_string(), toml::Value::String(name));
            entry.insert(
                "conversions".to_string(),
                toml::Value::Integer(data.conversions as i64),
            );
            entry.insert(
                "new_feedstocks".to_string(),
                toml::Value::Integer(data.new_feedstocks as i64),
            );
            entry.insert(
                "total_downloads".to_string(),
                toml::Value::Integer(data.total_downloads as i64),
            );

            // Find first and last contribution dates
            let mut dates: Vec<&str> = data
                .feedstocks
                .iter()
                .filter(|f| !f.date.is_empty())
                .map(|f| f.date.as_str())
                .collect();
            dates.sort();

            if let Some(first) = dates.first() {
                entry.insert(
                    "first_contribution".to_string(),
                    toml::Value::String(first.to_string()),
                );
            }
            if let Some(last) = dates.last() {
                entry.insert(
                    "last_contribution".to_string(),
                    toml::Value::String(last.to_string()),
                );
            }

            // Compute weekly activity from ALL feedstocks (before truncating)
            let weekly_activity = compute_weekly_activity(&data.feedstocks);
            let weekly_array: Vec<toml::Value> = weekly_activity
                .into_iter()
                .map(|(conv, new)| {
                    toml::Value::Array(vec![
                        toml::Value::Integer(conv as i64),
                        toml::Value::Integer(new as i64),
                    ])
                })
                .collect();
            entry.insert("weekly_activity".to_string(), toml::Value::Array(weekly_array));

            // Sort feedstocks by downloads (descending) and take top 10
            let mut sorted_feedstocks = data.feedstocks;
            sorted_feedstocks.sort_by(|a, b| b.downloads.cmp(&a.downloads));
            sorted_feedstocks.truncate(10);

            // Find top package
            if let Some(top) = sorted_feedstocks.first() {
                let mut top_pkg = toml::Table::new();
                top_pkg.insert("name".to_string(), toml::Value::String(top.name.clone()));
                top_pkg.insert(
                    "downloads".to_string(),
                    toml::Value::Integer(top.downloads as i64),
                );
                entry.insert("top_package".to_string(), toml::Value::Table(top_pkg));
            }

            // Add feedstocks list (top 10 by downloads)
            let feedstocks_array: Vec<toml::Value> = sorted_feedstocks
                .into_iter()
                .map(|f| {
                    let mut fs = toml::Table::new();
                    fs.insert("name".to_string(), toml::Value::String(f.name));
                    fs.insert(
                        "contribution_type".to_string(),
                        toml::Value::String(f.contribution_type),
                    );
                    fs.insert(
                        "downloads".to_string(),
                        toml::Value::Integer(f.downloads as i64),
                    );
                    fs.insert("date".to_string(), toml::Value::String(f.date));
                    toml::Value::Table(fs)
                })
                .collect();
            entry.insert("feedstocks".to_string(), toml::Value::Array(feedstocks_array));

            toml::Value::Table(entry)
        })
        .collect()
}
