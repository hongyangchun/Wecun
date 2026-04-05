use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const HISTORY_FILE_NAME: &str = "download_history.json";
const HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub uid: String,
    pub screen_name: String,
    pub output_dir: String,
    pub last_download: String,
    pub post_count: usize,
}

pub struct HistoryService;

impl HistoryService {
    pub fn new() -> Self {
        Self
    }

    fn history_path(app_data_dir: &Path) -> PathBuf {
        app_data_dir.join(HISTORY_FILE_NAME)
    }

    pub fn list(app_data_dir: &Path) -> Vec<HistoryEntry> {
        let mut entries = Self::read_entries(app_data_dir).unwrap_or_default();
        Self::sort_and_truncate(&mut entries);
        entries
    }

    pub fn save(&self, app_data_dir: &Path, entry: HistoryEntry) -> Result<(), io::Error> {
        let mut entries = Self::read_entries(app_data_dir).unwrap_or_default();
        entries.retain(|existing| existing.uid != entry.uid);
        entries.push(entry);
        Self::sort_and_truncate(&mut entries);
        Self::write_entries(app_data_dir, &entries)
    }

    pub fn delete(&self, app_data_dir: &Path, uid: &str) -> Result<(), io::Error> {
        let mut entries = Self::read_entries(app_data_dir).unwrap_or_default();
        entries.retain(|entry| entry.uid != uid);
        Self::sort_and_truncate(&mut entries);
        Self::write_entries(app_data_dir, &entries)
    }

    fn read_entries(app_data_dir: &Path) -> Result<Vec<HistoryEntry>, io::Error> {
        let path = Self::history_path(app_data_dir);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)?;
        let entries = serde_json::from_str::<Vec<HistoryEntry>>(&content).unwrap_or_default();
        Ok(entries)
    }

    fn write_entries(app_data_dir: &Path, entries: &[HistoryEntry]) -> Result<(), io::Error> {
        fs::create_dir_all(app_data_dir)?;
        let json = serde_json::to_string_pretty(entries)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        fs::write(Self::history_path(app_data_dir), json)
    }

    fn sort_and_truncate(entries: &mut Vec<HistoryEntry>) {
        entries.sort_by(|left, right| {
            parse_last_download(&right.last_download)
                .cmp(&parse_last_download(&left.last_download))
                .then_with(|| right.uid.cmp(&left.uid))
        });
        entries.truncate(HISTORY_LIMIT);
    }
}

fn parse_last_download(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;

    use super::{HistoryEntry, HistoryService};

    fn temp_app_data_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "weibo-history-test-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_entry(
        uid: &str,
        screen_name: &str,
        last_download: &str,
        post_count: usize,
    ) -> HistoryEntry {
        HistoryEntry {
            uid: uid.to_string(),
            screen_name: screen_name.to_string(),
            output_dir: format!("/tmp/{uid}"),
            last_download: last_download.to_string(),
            post_count,
        }
    }

    #[test]
    fn list_returns_most_recent_entries_first_and_limits_to_50() {
        let app_data_dir = temp_app_data_dir("list");
        let history_path = app_data_dir.join("download_history.json");
        let entries: Vec<_> = (0..55)
            .map(|index| {
                json!({
                    "uid": format!("uid-{index:02}"),
                    "screen_name": format!("用户{index}"),
                    "output_dir": format!("/tmp/{index}"),
                    "last_download": format!("2024-01-01T12:{index:02}:00Z"),
                    "post_count": index + 1,
                })
            })
            .collect();
        fs::write(&history_path, serde_json::to_string(&entries).unwrap()).unwrap();

        let history = HistoryService::list(&app_data_dir);

        assert_eq!(history.len(), 50);
        assert_eq!(history.first().unwrap().uid, "uid-54");
        assert_eq!(history.last().unwrap().uid, "uid-05");

        fs::remove_dir_all(&app_data_dir).ok();
    }

    #[test]
    fn save_replaces_existing_uid_and_keeps_newest_first() {
        let app_data_dir = temp_app_data_dir("save");
        let service = HistoryService::new();

        service
            .save(
                &app_data_dir,
                sample_entry("123", "旧名字", "2024-01-01T12:00:00Z", 12),
            )
            .unwrap();
        service
            .save(
                &app_data_dir,
                sample_entry("456", "第二个用户", "2024-01-02T12:00:00Z", 8),
            )
            .unwrap();
        service
            .save(
                &app_data_dir,
                sample_entry("123", "新名字", "2024-01-03T12:00:00Z", 99),
            )
            .unwrap();

        let history = HistoryService::list(&app_data_dir);

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].uid, "123");
        assert_eq!(history[0].screen_name, "新名字");
        assert_eq!(history[0].post_count, 99);
        assert_eq!(history[1].uid, "456");

        fs::remove_dir_all(&app_data_dir).ok();
    }

    #[test]
    fn delete_removes_matching_uid() {
        let app_data_dir = temp_app_data_dir("delete");
        let service = HistoryService::new();

        service
            .save(
                &app_data_dir,
                sample_entry("123", "用户一", "2024-01-01T12:00:00Z", 1),
            )
            .unwrap();
        service
            .save(
                &app_data_dir,
                sample_entry("456", "用户二", "2024-01-02T12:00:00Z", 2),
            )
            .unwrap();

        service.delete(&app_data_dir, "123").unwrap();

        let history = HistoryService::list(&app_data_dir);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].uid, "456");

        fs::remove_dir_all(&app_data_dir).ok();
    }
}
