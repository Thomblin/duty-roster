use std::fs::File;
use std::io::Write;

use crate::config::load_config;
use crate::dates::get_weekdays;
use crate::schedule::{Assignment, create_schedule};

/// Generate a schedule from a config file
pub async fn generate_schedule(config_path: String) -> Result<Vec<Assignment>, String> {
    match load_config(&config_path) {
        Ok(config) => {
            let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
            let (assignments, _) = create_schedule(&dates, &config);
            Ok(assignments)
        }
        Err(e) => Err(format!("Failed to load config: {e}")),
    }
}

/// Save schedule and summary to a file
pub async fn save_file(
    filename: String,
    csv_content: String,
    summary_content: String,
) -> Result<(), String> {
    match File::create(&filename) {
        Ok(file) => write_schedule_and_summary(file, &csv_content, &summary_content),
        Err(e) => Err(format!("Failed to create file: {e}")),
    }
}

fn write_schedule_and_summary(
    mut writer: impl Write,
    csv_content: &str,
    summary_content: &str,
) -> Result<(), String> {
    if let Err(e) = writer.write_all(csv_content.as_bytes()) {
        return Err(format!("Failed to write CSV content: {e}"));
    }

    if let Err(e) = writer.write_all(b"\n") {
        return Err(format!("Failed to write newline: {e}"));
    }

    match writer.write_all(summary_content.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write summary content: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_generate_schedule_success() {
        // Create a test config file
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_content = r#"
            [dates]
            from = "2025-09-01"
            to = "2025-09-30"
            weekdays = ["Mon", "Wed", "Fri"]
            exceptions = ["2025-09-05"]

            [places]
            places = ["Place A", "Place B"]

            [[group]]
            name = "Test"
            place = "Place A"

            [[group.members]]
            name = "Person1"

            [rules]
            sort = ["sortByLeastServices"]
            filter = []
        "#;
        std::fs::write(&config_path, config_content).unwrap();

        // Test the function
        let result = generate_schedule(config_path.to_string_lossy().to_string()).await;
        assert!(result.is_ok());
        let assignments = result.unwrap();
        assert!(!assignments.is_empty());
    }

    #[tokio::test]
    async fn test_generate_schedule_invalid_config() {
        // Test with non-existent config file
        let result = generate_schedule("non_existent_config.toml".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_save_file_success() {
        // Create temporary file for testing
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_string_lossy().to_string();

        // Test data
        let csv_content = "date,place,person\n2025-09-01,Place A,Person1";
        let summary_content = "Person1, total: 1, Mon: 1";

        // Test the function
        let result = save_file(
            file_path.clone(),
            csv_content.to_string(),
            summary_content.to_string(),
        )
        .await;
        assert!(result.is_ok());

        // Verify file content
        let content = std::fs::read_to_string(file_path).unwrap();
        assert!(content.contains(csv_content));
        assert!(content.contains(summary_content));
    }

    #[tokio::test]
    async fn test_save_file_invalid_path() {
        // Test with an invalid file path
        let file_path = "/invalid/path/that/should/not/exist/file.csv";
        let csv_content = "test";
        let summary_content = "test";

        let result = save_file(
            file_path.to_string(),
            csv_content.to_string(),
            summary_content.to_string(),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to create file"));
    }

    struct FailingWriter {
        fail_on_call: usize,
        calls: usize,
    }

    impl Write for FailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.calls == self.fail_on_call && !buf.is_empty() {
                return Err(io::Error::new(io::ErrorKind::Other, "write failed"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_write_schedule_and_summary_csv_write_error() {
        let writer = FailingWriter {
            fail_on_call: 1,
            calls: 0,
        };
        let result = write_schedule_and_summary(writer, "csv", "summary");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to write CSV content"));
    }

    #[test]
    fn test_write_schedule_and_summary_newline_write_error() {
        let writer = FailingWriter {
            fail_on_call: 2,
            calls: 0,
        };
        let result = write_schedule_and_summary(writer, "csv", "summary");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to write newline"));
    }

    #[test]
    fn test_write_schedule_and_summary_summary_write_error() {
        let writer = FailingWriter {
            fail_on_call: 3,
            calls: 0,
        };
        let result = write_schedule_and_summary(writer, "csv", "summary");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to write summary content"));
    }

    // Mock test to simulate write errors
    // Note: This is a bit tricky to test directly without mocking the File::write_all function
    // In a real-world scenario, you might use a mocking framework or dependency injection
    // For now, we'll just test the happy path and the file creation error
}
