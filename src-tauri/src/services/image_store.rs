use std::path::Path;

use crate::error::AppError;

pub struct ImageStoreService;

impl ImageStoreService {
    pub fn new() -> Self {
        Self
    }

    pub async fn download_image(
        &self,
        client: &reqwest::Client,
        url: &str,
        target_dir: &Path,
        filename: &str,
    ) -> Result<String, AppError> {
        let resp = client
            .get(url)
            .header("Referer", "https://weibo.com/")
            .send()
            .await
            .map_err(AppError::Network)?;

        let status = resp.status();
        if !status.is_success() {
            return Err(AppError::ApiError(format!("HTTP {status} for image: {url}")));
        }

        let bytes = resp.bytes().await.map_err(AppError::Network)?;

        if bytes.len() < 100 {
            return Err(AppError::ApiError(format!("Image too small ({} bytes): {}", bytes.len(), url)));
        }

        let dest = target_dir.join(filename);
        tokio::fs::write(&dest, &bytes).await?;

        Ok(filename.to_string())
    }

    pub async fn download_images(
        &self,
        client: &reqwest::Client,
        urls: &[(String, String)],
        target_dir: &Path,
    ) -> Result<Vec<(String, String)>, AppError> {
        let mut results = Vec::new();
        for (url, filename) in urls {
            match self.download_image(client, url, target_dir, filename).await {
                Ok(local) => results.push((url.clone(), local)),
                Err(e) => {
                    eprintln!("Failed to download {}: {}", url, e);
                }
            }
        }
        Ok(results)
    }
}
