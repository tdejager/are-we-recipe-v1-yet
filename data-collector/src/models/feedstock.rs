use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::RecipeType;

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedstockStats {
    pub total_feedstocks: u32,
    pub recipe_v1_count: u32,
    pub meta_yaml_count: u32,
    pub unknown_count: u32,
    pub last_updated: String,
    #[serde(default)]
    pub feedstock_states: BTreeMap<String, FeedstockEntry>,
    #[serde(default)]
    pub top_unconverted_by_downloads: Vec<TopFeedstock>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeedstockEntry {
    pub recipe_type: RecipeType,
    pub last_changed: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopFeedstock {
    pub name: String,
    pub downloads: u64,
    pub recipe_type: RecipeType,
}