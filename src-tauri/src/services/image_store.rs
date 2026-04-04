use std::path::Path;

use crate::error::AppError;

pub struct ImageStoreService;

impl ImageStoreService {
    pub fn new() -> Self {
        Self
    }

    pub async fn download_image(
        &self,
        url: &str,
        target_dir: &Path,
        filename: &str,
    ) -> Result<String, AppError> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(AppError::Network)?;

        let resp = client.get(url).send().await.map_err(AppError::Network)?;
        let bytes = resp.bytes().await.map_err(AppError::Network)?;

        let dest = target_dir.join(filename);
        tokio::fs::write(&dest, &bytes).await?;

        Ok(filename.to_string())
    }

    pub async fn download_images(
        &self,
        urls: &[(String, String)],
        target_dir: &Path,
    ) -> Result<Vec<(String, String)>, AppError> {
        let mut results = Vec::new();
        for (url, filename) in urls {
            match self.download_image(url, target_dir, filename).await {
                Ok(local) => results.push((url.clone(), local)),
                Err(e) => {
                    eprintln!("Failed to download {}: {}", url, e);
                }
            }
        }
        Ok(results)
    }
}
