use std::path::PathBuf;
use std::process::Command;

#[tauri::command]
pub fn open_path_in_explorer(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Choose a path before opening Explorer.".to_string());
    }

    let candidate = PathBuf::from(trimmed);
    if !candidate.exists() {
        return Err("Path no longer exists.".to_string());
    }

    #[cfg(windows)]
    {
        let mut command = Command::new("explorer.exe");
        if candidate.is_file() {
            command.arg("/select,").arg(&candidate);
        } else {
            command.arg(&candidate);
        }

        command.spawn().map_err(|error| error.to_string())?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let _ = candidate;
        Err("Windows Explorer handoff is only available on Windows.".to_string())
    }
}
