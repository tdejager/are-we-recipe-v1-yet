use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum RecipeType {
    #[serde(rename = "recipe_v1")]
    RecipeV1, // Has recipe.yaml
    #[serde(rename = "meta_yaml")]
    MetaYaml, // Has meta.yaml
    #[serde(rename = "unknown")]
    Unknown, // Neither or both
}

#[derive(Debug, Deserialize)]
pub struct NodeAttrsJson {
    pub feedstock_name: String,
    #[serde(rename = "conda-forge.yml", default)]
    pub conda_forge_yml: Option<CondaForgeYml>,
}

#[derive(Debug, Deserialize)]
pub struct CondaForgeYml {
    #[serde(default)]
    pub conda_build_tool: Option<String>,
}
