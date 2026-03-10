pub mod app;
pub mod assignment;
pub mod config;
pub mod state;
pub mod summary;
pub mod table;
pub mod utils;

// Re-export public items
pub use self::app::{CellPosition, DutyRosterApp, Message, Tab};
pub use self::config::{find_config_files, generate_filename};

/// Run the GUI application
pub fn run() -> iced::Result {
    iced::application(app::boot, app::update, app::view)
        .title("Duty Roster")
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reexports() {
        // This test just verifies that the re-exports are working correctly
        // by ensuring the types can be referenced

        // We can't easily test the run function directly as it would start the GUI
        // but we can verify that the types are exported correctly

        // Create a CellPosition to verify the type is exported
        let _position = CellPosition { row: 1, column: 1 };

        // Verify Tab enum is exported
        let _tab = Tab::Schedule;

        // Verify Message enum is exported (just create a simple variant)
        let _message = Message::MouseLeft;

        // We can't instantiate DutyRosterApp directly in a meaningful way here
        // but the fact that we can reference it means it's exported correctly
    }
}

#[cfg(test)]
mod tests_run {
    #[test]
    fn test_run_function_signature() {
        // We can't actually run the GUI in a test, but we can verify the function signature
        // by checking that it returns a Result<(), iced::Error>

        // Create a mock function with the same signature
        fn mock_run() -> iced::Result {
            Ok(())
        }

        // If this compiles, it means the signatures match
        let _: fn() -> iced::Result = mock_run;
        let _: fn() -> iced::Result = super::run;

        // Just assert true to have a passing test
        assert!(true);
    }

}
