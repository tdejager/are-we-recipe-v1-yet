use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::{NodeAttrsJson, RecipeType};

/// Parses a JSON file containing node attributes into a `NodeAttrsJson` struct.
pub fn parse_node_attrs_file(path: &Path) -> Result<NodeAttrsJson> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?;

    let node_data: NodeAttrsJson = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON in file: {:?}", path))?;

    Ok(node_data)
}

/// Determines the recipe type based on the `conda_build_tool` field in the node data.
pub fn determine_recipe_type_from_node(node_data: &NodeAttrsJson) -> RecipeType {
    // Check if conda_build_tool is set to rattler-build in conda-forge.yml
    if let Some(conda_forge_yml) = &node_data.conda_forge_yml {
        if let Some(conda_build_tool) = &conda_forge_yml.conda_build_tool {
            if conda_build_tool == "rattler-build" {
                return RecipeType::RecipeV1;
            }
        }
    }

    // If no rattler-build conda_build_tool found, it's using conda-build (legacy)
    RecipeType::MetaYaml
}
