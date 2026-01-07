pub mod app;
pub mod assignment;
pub mod config;
pub mod state;
pub mod summary;
pub mod table;
pub mod utils;

// Re-export public items
pub use self::app::{DutyRosterApp, Message, Tab, CellPosition};
pub use self::config::{find_config_files, generate_filename};

/// Run the GUI application
pub fn run() -> iced::Result {
    use iced::Application;
    self::app::DutyRosterApp::run(iced::Settings::default())
}
