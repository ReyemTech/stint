use crate::commands::AppError;
use crate::windows;
use tauri::AppHandle;

fn wrap(e: tauri::Error) -> AppError {
    AppError {
        kind: "window".into(),
        message: e.to_string(),
    }
}

#[tauri::command]
pub async fn show_main_window(app: AppHandle) -> Result<(), AppError> {
    // Hide the popover first so it disappears immediately when the user
    // chooses to open the main window — avoids any focus-race where the
    // JS-side hide gets dropped after the main window takes focus.
    windows::hide_popover(&app).map_err(wrap)?;
    windows::show_main(&app).map_err(wrap)?;
    Ok(())
}
