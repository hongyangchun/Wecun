use crate::services::profile::ai_key;

/// Whether the user has set a key for the AI analysis.
#[tauri::command]
pub fn get_ai_key_status(app: tauri::AppHandle) -> bool {
    ai_key::read(&app).is_some()
}

#[tauri::command]
pub fn set_ai_key(key: String, app: tauri::AppHandle) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("密钥不能为空".to_string());
    }
    ai_key::save(&app, &key).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_ai_key(app: tauri::AppHandle) {
    ai_key::clear(&app);
}
