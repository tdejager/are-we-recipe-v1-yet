use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::RecipeType;

/// Type of contribution for Recipe v1 feedstocks
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContributionType {
    /// Converted existing meta.yaml to recipe.yaml
    Conversion,
    /// Created new feedstock with recipe.yaml
    NewFeedstock,
}

/// Attribution information for Recipe v1 feedstocks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Attribution {
    pub contribution_type: ContributionType,
    /// GitHub handles of contributors
    pub contributors: Vec<String>,
    /// Date when recipe.yaml was added (ISO 8601)
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}

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
    /// Attribution for Recipe v1 feedstocks (who converted/created it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<Attribution>,
    /// Download count for this feedstock
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloads: Option<u64>,
    /// Cached data from batch query (step 1-2) for resuming attribution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_commit_cache: Option<RecipeCommitCache>,
}

/// Cached commit info from batch query, saved to allow resuming attribution
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecipeCommitCache {
    pub sha: String,
    pub message: String,
    pub date: String,
    pub author_login: Option<String>,
    pub author_name: String,
    pub author_email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopFeedstock {
    pub name: String,
    pub downloads: u64,
    pub recipe_type: RecipeType,
}
