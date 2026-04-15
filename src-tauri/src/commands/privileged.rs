#[tauri::command]
pub fn get_privileged_cleanup_capability() -> elevation_helper::PrivilegedCleanupCapability {
    elevation_helper::privileged_cleanup_capability()
}
