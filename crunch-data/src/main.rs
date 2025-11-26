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

/// Extract the 10 most recently updated Recipe v1 feedstocks
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
                    date.as_str()
                        .map(|date_str| (name.clone(), date_str.to_string()))
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by last updated date (most recent first)
    recent_feedstocks.sort_by(|(_, a), (_, b)| b.cmp(a));

    // Take the 10 most recent
    recent_feedstocks.truncate(10);

    // Create a new table for the recent feedstocks
    let mut recent_table = toml::Table::new();
    for (name, date) in recent_feedstocks {
        recent_table.insert(name, toml::Value::String(date));
    }

    recent_table
}

/// Extract top contributors from attribution data
fn extract_top_contributors(feedstocks_table: &toml::Table) -> Vec<toml::Value> {
    // Aggregate contributions by contributor
    let mut contributor_stats: HashMap<String, (u32, u32)> = HashMap::new(); // (conversions, new_feedstocks)

    for (_name, state) in feedstocks_table.iter() {
        if let Some(attribution) = state.get("attribution").and_then(|a| a.as_table()) {
            let contribution_type = attribution
                .get("contribution_type")
                .and_then(|t| t.as_str())
                .unwrap_or("");

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
                let entry = contributor_stats.entry(contributor).or_insert((0, 0));
                match contribution_type {
                    "conversion" => entry.0 += 1,
                    "new_feedstock" => entry.1 += 1,
                    _ => {}
                }
            }
        }
    }

    // Sort by total contributions (descending)
    let mut sorted: Vec<_> = contributor_stats
        .into_iter()
        .map(|(name, (conversions, new_feedstocks))| (name, conversions, new_feedstocks))
        .collect();

    sorted.sort_by(|a, b| {
        let total_a = a.1 + a.2;
        let total_b = b.1 + b.2;
        total_b.cmp(&total_a)
    });

    // Take top 50 and convert to TOML
    sorted
        .into_iter()
        .take(50)
        .map(|(name, conversions, new_feedstocks)| {
            let mut entry = toml::Table::new();
            entry.insert("name".to_string(), toml::Value::String(name));
            entry.insert(
                "conversions".to_string(),
                toml::Value::Integer(conversions as i64),
            );
            entry.insert(
                "new_feedstocks".to_string(),
                toml::Value::Integer(new_feedstocks as i64),
            );
            toml::Value::Table(entry)
        })
        .collect()
}
