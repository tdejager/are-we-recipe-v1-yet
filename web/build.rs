use std::fs;

fn main() {
    let input_path = "../feedstock-stats.toml";
    
    if let Ok(content) = fs::read_to_string(input_path) {
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
            
            let summary_toml = toml::to_string(&summary).unwrap();
            fs::write("src/stats.toml", summary_toml).expect("Failed to write summary");
        }
    }
    
    println!("cargo:rerun-if-changed={}", input_path);
}