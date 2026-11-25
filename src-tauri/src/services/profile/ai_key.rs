use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use crate::error::AppError;

/// The user's key for the AI analysis, kept as a file in the app's data directory.
const FILE_NAME: &str = "ai-key";

fn key_path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

/// The stored key, or `None` when the user has not set one.
pub fn read_from(dir: &Path) -> Option<String> {
    let key = fs::read_to_string(key_path(dir)).ok()?.trim().to_string();
    (!key.is_empty()).then_some(key)
}

/// Stores the key where only its owner can read it.
pub fn save_to(dir: &Path, key: &str) -> Result<(), AppError> {
    fs::create_dir_all(dir).map_err(AppError::Io)?;
    let staging = dir.join(format!("{FILE_NAME}.tmp"));
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Set before the first byte is written, not afterwards: the key never exists world-readable.
        options.mode(0o600);
    }
    let mut file = options.open(&staging).map_err(AppError::Io)?;
    file.write_all(key.trim().as_bytes()).map_err(AppError::Io)?;
    drop(file);
    fs::rename(&staging, key_path(dir)).map_err(AppError::Io)
}

pub fn clear_from(dir: &Path) {
    let _ = fs::remove_file(key_path(dir));
}

/// The key this run uses: the one the user set in the app. `DEEPSEEK_API_KEY` is honoured in debug
/// builds only. In a release it would override the key the user set, and outlive "清除密钥".
pub fn read<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    #[cfg(debug_assertions)]
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Some(key);
        }
    }
    read_from(&app.path().app_data_dir().ok()?)
}

pub fn save<R: Runtime>(app: &AppHandle<R>, key: &str) -> Result<(), AppError> {
    let dir = app.path().app_data_dir().map_err(|error| AppError::ApiError(error.to_string()))?;
    save_to(&dir, key)
}

pub fn clear<R: Runtime>(app: &AppHandle<R>) {
    if let Ok(dir) = app.path().app_data_dir() {
        clear_from(&dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_stored_trimmed_and_readable_by_its_owner_only() {
        let dir = std::env::temp_dir().join(format!("wecun-ai-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(read_from(&dir), None, "nothing is stored yet");

        save_to(&dir, "  sk-test\n").unwrap();
        assert_eq!(read_from(&dir).as_deref(), Some("sk-test"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(FILE_NAME)).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "other users on this machine must not be able to read the key");
        }

        clear_from(&dir);
        assert_eq!(read_from(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
