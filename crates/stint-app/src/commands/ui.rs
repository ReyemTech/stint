use crate::commands::AppError;
use crate::windows;
use tauri::AppHandle;

#[tauri::command]
pub async fn show_main_window(app: AppHandle) -> Result<(), AppError> {
    windows::show_main(&app).map_err(|e| AppError {
        kind: "window".into(),
        message: e.to_string(),
    })
}
