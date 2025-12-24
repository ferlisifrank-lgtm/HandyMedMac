use crate::medical_vocab::MedicalVocabulary;

#[tauri::command]
#[specta::specta]
pub fn get_custom_vocab_path() -> Result<String, String> {
    let path = MedicalVocabulary::ensure_custom_vocab_file_exists()?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
#[specta::specta]
pub fn open_custom_vocab_file() -> Result<(), String> {
    let path = MedicalVocabulary::ensure_custom_vocab_file_exists()?;
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    Ok(())
}