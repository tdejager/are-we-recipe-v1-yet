use anyhow::{Context, Result};
use std::collections::HashMap;

pub async fn fetch_download_counts() -> Result<HashMap<String, u64>> {
    let url = "https://storage.googleapis.com/download-count-cache/top_downloads_conda-forge.json";
    let client = reqwest::Client::new();
    
    let response = client.get(url).send().await
        .context("Failed to fetch download counts")?;
    
    let download_data: Vec<[serde_json::Value; 2]> = response.json().await
        .context("Failed to parse download counts JSON")?;
    
    let mut download_counts = HashMap::new();
    
    for entry in download_data {
        if let (Some(package_name), Some(count)) = (entry[0].as_str(), entry[1].as_u64()) {
            // Convert package name to feedstock name format with special mappings
            let feedstock_name = match package_name {
                "libzlib" => "zlib-feedstock".to_string(),
                "libblas" => "blas-feedstock".to_string(), 
                _ => format!("{}-feedstock", package_name)
            };
            
            // Only insert if this is a higher count or the feedstock doesn't exist yet
            // This prioritizes libzlib over zlib if both map to zlib-feedstock
            if let Some(&existing_count) = download_counts.get(&feedstock_name) {
                if count > existing_count {
                    download_counts.insert(feedstock_name, count);
                }
            } else {
                download_counts.insert(feedstock_name, count);
            }
        }
    }
    
    Ok(download_counts)
}