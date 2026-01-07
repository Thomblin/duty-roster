mod app;
mod assignment;
mod config;
mod state;
mod summary;
mod table;
mod utils;

// Re-export public items
pub use self::app::{DutyRosterApp, Message, Tab, CellPosition};

// Re-export for external use
pub use self::config::{find_config_files, generate_filename, create_config_selector};

/// Run the GUI application
pub fn run() -> iced::Result {
    use iced::Application;
    DutyRosterApp::run(iced::Settings::default())
}