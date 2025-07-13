use std::collections::{BTreeMap, HashMap};

use crate::models::{FeedstockEntry, RecipeType, TopFeedstock};

/// Calculates the top unconverted feedstocks based on their download counts.
pub fn calculate_top_unconverted_feedstocks(
    feedstock_states: &BTreeMap<String, FeedstockEntry>,
    download_counts: &HashMap<String, u64>,
    limit: usize,
) -> Vec<TopFeedstock> {
    let mut unconverted_with_downloads: Vec<TopFeedstock> = feedstock_states
        .iter()
        .filter(|(_, entry)| entry.recipe_type != RecipeType::RecipeV1)
        .filter_map(|(name, entry)| {
            download_counts.get(name).map(|&downloads| TopFeedstock {
                name: name.clone(),
                downloads,
                recipe_type: entry.recipe_type.clone(),
            })
        })
        .collect();

    // Sort by downloads in descending order
    unconverted_with_downloads.sort_by(|a, b| b.downloads.cmp(&a.downloads));

    // Take top N
    unconverted_with_downloads.into_iter().take(limit).collect()
}
