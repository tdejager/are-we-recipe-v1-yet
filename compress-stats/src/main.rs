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

            // Walk over the feedstocks to generate the 10 feedstocks that were most recently updated to recipe_v1
            if let Some(feedstocks) = toml_data.get("feedstock_states") {
                if let Some(feedstocks_table) = feedstocks.as_table() {
                    let mut recent_feedstocks: Vec<_> = feedstocks_table
                        .iter()
                        .filter_map(|(name, state)| {
                            // Only include recipe_v1 feedstocks
                            if state
                                .get("recipe_type")
                                .and_then(|recipe_type| {
                                    recipe_type.as_str().map(|s| s == "recipe_v1")
                                })
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

                    summary.insert(
                        "recently_updated".to_string(),
                        toml::Value::Table(recent_table),
                    );
                }
            }

            // Include top unconverted feedstocks by downloads
            if let Some(top_unconverted) = toml_data.get("top_unconverted_by_downloads") {
                summary.insert("top_unconverted_by_downloads".to_string(), top_unconverted.clone());
            }

            // Write the complete summary with recently_updated data
            let summary_toml = toml::to_string(&summary).unwrap();
            fs::write(&output_path, summary_toml).expect("Failed to write summary");
            println!("Compressed feedstock stats written to {}", output_path.display());
        }
    }
}