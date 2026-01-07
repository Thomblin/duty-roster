use iced::{Element, Length};
use iced::widget::{button, pick_list, row, text};
use std::fs;
use std::path::Path;
use chrono::Local;

/// Find all TOML config files in the current directory and subdirectories
pub async fn find_config_files() -> Result<Vec<String>, String> {
    // Look for config files in the current directory and subdirectories
    let mut config_files = Vec::new();
    
    // Files to exclude
    let excluded_files = ["Cargo.toml"];
    
    // Start with the current directory
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    // Skip excluded files
                    if excluded_files.contains(&file_name) {
                        continue;
                    }
                    
                    if let Some(path_str) = path.to_str() {
                        config_files.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    // Add test directory if it exists
    if let Ok(entries) = fs::read_dir("test") {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    // Skip excluded files (unlikely to be in test directory, but check anyway)
                    if excluded_files.contains(&file_name) {
                        continue;
                    }
                    
                    if let Some(path_str) = path.to_str() {
                        config_files.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    if config_files.is_empty() {
        return Err("No config files found".to_string());
    }
    
    Ok(config_files)
}

/// Generate a filename for saving the schedule based on the config path
pub fn generate_filename(config_path: String) -> String {
    let path = Path::new(&config_path);
    let file_stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("schedule");
    
    // Include date and time (hours and minutes) in the filename
    let datetime_stamp = Local::now().format("%Y_%m_%d_%H_%M").to_string();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    
    let out_path = parent.join(format!("{file_stem}_{datetime_stamp}.csv"));
    out_path.to_string_lossy().to_string()
}

/// Create a UI row with config dropdown and refresh button
pub fn create_config_selector<'a, Message>(
    config_files: &[String],
    selected_config: &Option<String>,
    on_config_selected: impl Fn(String) -> Message + 'static,
    on_refresh: Message,
) -> Element<'a, Message> 
where
    Message: Clone + 'static,
{
    let refresh_button = button(text("Refresh").size(14)).on_press(on_refresh);
    
    if config_files.is_empty() {
        row![text("No config files found").size(14), refresh_button].into()
    } else {
        row![
            text("Select config file:").size(14).width(Length::Fill),
            pick_list(
                config_files.to_vec(),
                selected_config.clone(),
                on_config_selected
            )
            .width(Length::Fill),
            refresh_button
        ].into()
    }
}
