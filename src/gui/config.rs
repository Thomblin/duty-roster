use chrono::Local;
use iced::widget::{button, pick_list, row, text};
use iced::{Element, Length};
use std::fs;
use std::path::Path;

/// Find all TOML config files in the current directory and subdirectories
pub async fn find_config_files() -> Result<Vec<String>, String> {
    find_config_files_in(Path::new("."))
}

fn find_config_files_in(root: &Path) -> Result<Vec<String>, String> {
    let mut config_files = Vec::new();

    let excluded_files = ["Cargo.toml", "deny.toml", "cargo-deny.toml"];

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|ext| ext == "toml")
                && let Some(file_name) = path.file_name().and_then(|f| f.to_str())
                && !excluded_files.contains(&file_name)
            {
                config_files.push(path.to_string_lossy().to_string());
            }
        }
    }

    let test_dir = root.join("test");
    if let Ok(entries) = fs::read_dir(test_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|ext| ext == "toml")
                && let Some(file_name) = path.file_name().and_then(|f| f.to_str())
                && !excluded_files.contains(&file_name)
            {
                config_files.push(path.to_string_lossy().to_string());
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
    let file_stem = path
        .file_stem()
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
        ]
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_generate_filename() {
        // Test with a simple path
        let config_path = "test/config.toml".to_string();
        let filename = generate_filename(config_path);

        // Check that the filename has the expected format
        let file_stem = Path::new("test/config")
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let today = Local::now().format("%Y_%m_%d").to_string();
        assert!(filename.contains(file_stem));
        assert!(filename.contains(&today));
        assert!(filename.ends_with(".csv"));

        // Test with a path containing parent directories
        let config_path = "/some/path/to/settings.toml".to_string();
        let filename = generate_filename(config_path);
        assert!(filename.contains("settings"));
        assert!(filename.contains(&today));
        assert!(filename.ends_with(".csv"));
    }

    #[test]
    fn test_find_config_files() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let config1_path = temp_dir.path().join("config1.toml");
        let config2_path = temp_dir.path().join("config2.toml");
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        let non_toml_path = temp_dir.path().join("file.txt");

        fs::write(&config1_path, "test content").unwrap();
        fs::write(&config2_path, "test content").unwrap();
        fs::write(&cargo_toml_path, "test content").unwrap();
        fs::write(&non_toml_path, "test content").unwrap();

        // Create test directory structure
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();
        let test_config_path = test_dir.join("test_config.toml");
        fs::write(&test_config_path, "test content").unwrap();

        // Test the function
        let result = find_config_files_in(temp_dir.path());
        assert!(result.is_ok());

        let config_files = result.unwrap();
        assert!(config_files.len() >= 2); // At least config1.toml and config2.toml

        // Check that Cargo.toml is excluded
        assert!(!config_files.iter().any(|path| path.contains("Cargo.toml")));

        // Check that non-TOML files are excluded
        assert!(!config_files.iter().any(|path| path.contains("file.txt")));

        // Check that test directory files are included
        assert!(
            config_files
                .iter()
                .any(|path| path.contains("test_config.toml"))
        );

    }

    #[test]
    fn test_find_config_files_no_configs() {
        let temp_dir = TempDir::new().unwrap();

        let result = find_config_files_in(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No config files found"));
    }

    #[test]
    fn test_create_config_selector() {
        // Test with empty config files
        #[derive(Clone)]
        enum TestMessage {
            ConfigSelected(String),
            Refresh,
        }

        let msg = TestMessage::ConfigSelected("x".to_string());
        if let TestMessage::ConfigSelected(s) = msg {
            assert_eq!(s, "x");
        }

        let config_files: Vec<String> = vec![];
        let selected_config: Option<String> = None;

        // Just verify that the function returns without panicking
        let _element = create_config_selector(
            &config_files,
            &selected_config,
            TestMessage::ConfigSelected,
            TestMessage::Refresh,
        );

        // Test with non-empty config files
        let config_files = vec!["config1.toml".to_string(), "config2.toml".to_string()];
        let selected_config = Some("config1.toml".to_string());

        let _element = create_config_selector(
            &config_files,
            &selected_config,
            TestMessage::ConfigSelected,
            TestMessage::Refresh,
        );

        // Just ensure the test passes without panicking
        assert!(true);
    }
}
