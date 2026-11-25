use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::error::AppError;
use crate::state::AppState;

const IMAGE_DOWNLOAD_CONCURRENCY: usize = 4;
const IMAGE_MAX_RETRIES: u32 = 2;
const MIN_IMAGE_BYTES: u64 = 100;

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
        let dest = target_dir.join(filename);
        if let Ok(meta) = tokio::fs::metadata(&dest).await {
            if meta.is_file() && meta.len() >= MIN_IMAGE_BYTES {
                return Ok(filename.to_string());
            }
        }

        let mut last_err = None;
        for attempt in 0..=IMAGE_MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))).await;
            }

            match self.fetch_and_save(client, url, &dest, filename).await {
                Ok(saved) => return Ok(saved),
                Err(err) => {
                    let retryable = matches!(err, AppError::Network(_));
                    last_err = Some(err);
                    if !retryable {
                        break;
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| AppError::ApiError(format!("Failed to download image: {url}"))))
    }

    async fn fetch_and_save(
        &self,
        client: &reqwest::Client,
        url: &str,
        dest: &Path,
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

        if (bytes.len() as u64) < MIN_IMAGE_BYTES {
            return Err(AppError::ApiError(format!(
                "Image too small ({} bytes): {}",
                bytes.len(),
                url
            )));
        }

        tokio::fs::write(dest, &bytes).await?;

        Ok(filename.to_string())
    }

    /// Download a batch of `(url, filename)` images concurrently with bounded concurrency,
    /// skip-if-exists, retries, and cancellation checks.
    pub async fn download_images_concurrent<F>(
        &self,
        client: &reqwest::Client,
        items: Vec<(String, String)>,
        target_dir: &Path,
        state: &AppState,
        mut on_item_done: F,
    ) -> Result<Vec<(String, String)>, AppError>
    where
        F: FnMut(bool),
    {
        let semaphore = Arc::new(Semaphore::new(IMAGE_DOWNLOAD_CONCURRENCY));
        let target_dir = Arc::new(PathBuf::from(target_dir));
        let mut join_set = JoinSet::new();
        let mut results = Vec::with_capacity(items.len());

        let mut iter = items.into_iter();
        let spawn_next = |set: &mut JoinSet<(String, Option<String>)>,
                              url: String,
                              filename: String| {
            let sem = Arc::clone(&semaphore);
            let dir = Arc::clone(&target_dir);
            let client = client.clone();
            set.spawn(async move {
                let _permit = match sem.acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => return (url, None),
                };
                let svc = ImageStoreService::new();
                match svc.download_image(&client, &url, &dir, &filename).await {
                    Ok(local) => (url, Some(local)),
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        eprintln!("Failed to download {}: {}", url, _e);
                        (url, None)
                    }
                }
            });
        };

        // Prime initial window up to concurrency * 2 so cancellation stays responsive
        for _ in 0..(IMAGE_DOWNLOAD_CONCURRENCY * 2) {
            if state.is_cancelled() {
                join_set.abort_all();
                return Err(AppError::Cancelled);
            }
            if let Some((url, filename)) = iter.next() {
                spawn_next(&mut join_set, url, filename);
            } else {
                break;
            }
        }

        while let Some(joined) = join_set.join_next().await {
            if state.is_cancelled() {
                join_set.abort_all();
                return Err(AppError::Cancelled);
            }

            if let Ok((url, local)) = joined {
                let ok = local.is_some();
                if let Some(filename) = local {
                    results.push((url, filename));
                }
                on_item_done(ok);
            }

            if let Some((url, filename)) = iter.next() {
                spawn_next(&mut join_set, url, filename);
            }
        }

        Ok(results)
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
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("Failed to download {}: {}", url, _e);
                }
            }
        }
        Ok(results)
    }
}
