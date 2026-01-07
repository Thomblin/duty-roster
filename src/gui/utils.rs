use std::fs::File;
use std::io::Write;

use crate::config::load_config;
use crate::dates::get_weekdays;
use crate::schedule::{create_schedule, Assignment};

/// Generate a schedule from a config file
pub async fn generate_schedule(config_path: String) -> Result<Vec<Assignment>, String> {
    match load_config(&config_path) {
        Ok(config) => {
            let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
            let (assignments, _) = create_schedule(&dates, &config);
            Ok(assignments)
        },
        Err(e) => Err(format!("Failed to load config: {}", e)),
    }
}

/// Save schedule and summary to a file
pub async fn save_file(filename: String, csv_content: String, summary_content: String) -> Result<(), String> {
    match File::create(&filename) {
        Ok(mut file) => {
            // Write CSV content
            if let Err(e) = file.write_all(csv_content.as_bytes()) {
                return Err(format!("Failed to write CSV content: {}", e));
            }
            
            // Add a newline between CSV and summary
            if let Err(e) = file.write_all(b"\n") {
                return Err(format!("Failed to write newline: {}", e));
            }
            
            // Write summary content
            match file.write_all(summary_content.as_bytes()) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to write summary content: {}", e)),
            }
        },
        Err(e) => Err(format!("Failed to create file: {}", e)),
    }
}
