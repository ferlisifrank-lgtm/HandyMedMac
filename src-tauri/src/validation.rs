/// Input validation utilities for user-provided data
///
/// This module provides comprehensive validation for all user inputs to prevent:
/// - Injection attacks (command injection, path traversal, SQL injection)
/// - Denial of Service (excessive memory usage, infinite loops)
/// - Invalid characters (null bytes, control characters)
/// - Format violations (malformed shortcuts, invalid IDs)
///
/// All validation functions return `Result<(), String>` where:
/// - `Ok(())` means the input is valid and safe to use
/// - `Err(String)` contains a human-readable error message
use std::path::Path;

/// Maximum length for a custom word to prevent memory issues
const MAX_CUSTOM_WORD_LENGTH: usize = 100;

/// Maximum number of custom words to prevent performance degradation
const MAX_CUSTOM_WORDS_COUNT: usize = 10_000;

/// Maximum length for shortcut binding string
const MAX_SHORTCUT_LENGTH: usize = 100;

/// Validates a custom word for safety and correctness
///
/// # Arguments
/// * `word` - The word to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(String)` with error message if invalid
pub fn validate_custom_word(word: &str) -> Result<(), String> {
    // Check for empty strings
    if word.trim().is_empty() {
        return Err("Custom word cannot be empty".to_string());
    }

    // Check length
    if word.len() > MAX_CUSTOM_WORD_LENGTH {
        return Err(format!(
            "Custom word too long (max {} characters)",
            MAX_CUSTOM_WORD_LENGTH
        ));
    }

    // Check for control characters or non-printable characters
    if word.chars().any(|c| c.is_control()) {
        return Err("Custom word contains invalid control characters".to_string());
    }

    // Check for dangerous characters that could cause injection
    if word.contains('\0') {
        return Err("Custom word contains null byte".to_string());
    }

    Ok(())
}

/// Validates a list of custom words
///
/// # Arguments
/// * `words` - The list of words to validate
///
/// # Returns
/// * `Ok(())` if all words are valid
/// * `Err(String)` with error message if any word is invalid
pub fn validate_custom_words(words: &[String]) -> Result<(), String> {
    // Check count
    if words.len() > MAX_CUSTOM_WORDS_COUNT {
        return Err(format!(
            "Too many custom words (max {})",
            MAX_CUSTOM_WORDS_COUNT
        ));
    }

    // Validate each word
    for (i, word) in words.iter().enumerate() {
        validate_custom_word(word).map_err(|e| format!("Word {}: {}", i + 1, e))?;
    }

    Ok(())
}

/// Validates a shortcut binding string
///
/// # Arguments
/// * `shortcut` - The shortcut string to validate (e.g., "Ctrl+Shift+A")
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(String)` with error message if invalid
pub fn validate_shortcut(shortcut: &str) -> Result<(), String> {
    // Check for empty strings
    if shortcut.trim().is_empty() {
        return Err("Shortcut cannot be empty".to_string());
    }

    // Check length
    if shortcut.len() > MAX_SHORTCUT_LENGTH {
        return Err(format!(
            "Shortcut too long (max {} characters)",
            MAX_SHORTCUT_LENGTH
        ));
    }

    // Check for control characters
    if shortcut.chars().any(|c| c.is_control()) {
        return Err("Shortcut contains invalid control characters".to_string());
    }

    // Check for dangerous characters
    if shortcut.contains('\0') || shortcut.contains('\n') || shortcut.contains('\r') {
        return Err("Shortcut contains invalid characters".to_string());
    }

    // Validate format: should contain at least one non-modifier key
    let modifiers = [
        "ctrl", "control", "shift", "alt", "option", "meta", "command", "cmd", "super", "win",
        "windows",
    ];

    let parts: Vec<&str> = shortcut.split('+').collect();

    // Check if there are any parts at all
    if parts.is_empty() {
        return Err("Shortcut has no keys".to_string());
    }

    // Check for empty parts (like "Ctrl++A")
    if parts.iter().any(|p| p.trim().is_empty()) {
        return Err("Shortcut has empty key components".to_string());
    }

    let has_non_modifier = parts
        .iter()
        .any(|part| !modifiers.contains(&part.trim().to_lowercase().as_str()));

    if !has_non_modifier {
        return Err("Shortcut must contain at least one non-modifier key".to_string());
    }

    Ok(())
}

/// Validates a file path for safety
///
/// # Arguments
/// * `path` - The file path to validate
/// * `must_exist` - Whether the path must exist
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(String)` with error message if invalid
#[allow(dead_code)]
pub fn validate_file_path(path: &str, must_exist: bool) -> Result<(), String> {
    // Check for empty strings
    if path.trim().is_empty() {
        return Err("File path cannot be empty".to_string());
    }

    // Check for null bytes
    if path.contains('\0') {
        return Err("File path contains null byte".to_string());
    }

    // Check for path traversal attempts (basic check)
    if path.contains("..") && !Path::new(path).is_absolute() {
        return Err("Relative paths with '..' are not allowed".to_string());
    }

    // If must exist, verify it
    if must_exist && !Path::new(path).exists() {
        return Err(format!("File path does not exist: {}", path));
    }

    Ok(())
}

/// Validates a model ID to prevent injection
///
/// # Arguments
/// * `model_id` - The model ID to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(String)` with error message if invalid
pub fn validate_model_id(model_id: &str) -> Result<(), String> {
    // Check for empty strings
    if model_id.trim().is_empty() {
        return Err("Model ID cannot be empty".to_string());
    }

    // Check length (reasonable maximum)
    if model_id.len() > 100 {
        return Err("Model ID too long".to_string());
    }

    // Only allow alphanumeric, dash, underscore, and dot
    if !model_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("Model ID contains invalid characters (only alphanumeric, -, _, . allowed)".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_custom_word() {
        // Valid words
        assert!(validate_custom_word("hello").is_ok());
        assert!(validate_custom_word("COVID-19").is_ok());
        assert!(validate_custom_word("François").is_ok());

        // Invalid words
        assert!(validate_custom_word("").is_err());
        assert!(validate_custom_word("   ").is_err());
        assert!(validate_custom_word(&"x".repeat(101)).is_err());
        assert!(validate_custom_word("hello\0world").is_err());
        assert!(validate_custom_word("hello\nworld").is_err());
    }

    #[test]
    fn test_validate_custom_words() {
        // Valid lists
        assert!(validate_custom_words(&vec!["hello".to_string(), "world".to_string()]).is_ok());
        assert!(validate_custom_words(&vec![]).is_ok());

        // Invalid lists
        let too_many: Vec<String> = (0..10_001).map(|i| format!("word{}", i)).collect();
        assert!(validate_custom_words(&too_many).is_err());

        let with_invalid = vec!["hello".to_string(), "".to_string()];
        assert!(validate_custom_words(&with_invalid).is_err());
    }

    #[test]
    fn test_validate_shortcut() {
        // Valid shortcuts
        assert!(validate_shortcut("Ctrl+A").is_ok());
        assert!(validate_shortcut("Ctrl+Shift+A").is_ok());
        assert!(validate_shortcut("Command+Option+F").is_ok());

        // Invalid shortcuts
        assert!(validate_shortcut("").is_err());
        assert!(validate_shortcut("Ctrl+Shift").is_err()); // Only modifiers
        assert!(validate_shortcut("Ctrl++A").is_err()); // Empty component
        assert!(validate_shortcut("Ctrl\0A").is_err()); // Null byte
    }

    #[test]
    fn test_validate_model_id() {
        // Valid IDs
        assert!(validate_model_id("whisper-small").is_ok());
        assert!(validate_model_id("parakeet_v2").is_ok());
        assert!(validate_model_id("model.1.0").is_ok());

        // Invalid IDs
        assert!(validate_model_id("").is_err());
        assert!(validate_model_id("model/path").is_err()); // Slash not allowed
        assert!(validate_model_id("model id").is_err()); // Space not allowed
    }
}
