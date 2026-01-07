use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Application, Command, Element, Length, Theme, executor};

use crate::config::load_config;
use crate::csv::assignments_to_csv;
use crate::dates::get_weekdays;
use crate::schedule::{Assignment, create_schedule};

use super::state::AppState;
use super::summary;
use super::table;
use super::utils;

/// Tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Schedule,
    Summary,
}

/// Cell position in the table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPosition {
    pub row: usize,
    pub column: usize,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    ConfigSelected(String),
    RefreshConfigList,
    GenerateSchedule,
    SaveScheduleWithDate,
    SaveSchedule(String), // filename only
    ConfigsLoaded(Result<Vec<String>, String>),
    ScheduleGenerated(Result<Vec<Assignment>, String>), // assignments only
    ScheduleSaved(Result<(), String>),
    TabSelected(Tab),
    CellClicked(CellPosition),
    CellHovered(CellPosition),
    MouseEntered(CellPosition),
    MouseLeft,
    Error(String),
    CheckMessageExpiry,
    ShowSuccessMessage(String),
}

/// Main application
pub struct DutyRosterApp {
    state: AppState,
}

impl Application for DutyRosterApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                state: AppState::new(),
            },
            Command::perform(
                async { crate::gui::find_config_files().await },
                Message::ConfigsLoaded,
            ),
        )
    }

    fn title(&self) -> String {
        String::from("Duty Roster")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ConfigsLoaded(Ok(files)) => {
                self.state.config_files = files;
                if !self.state.config_files.is_empty() && self.state.selected_config.is_none() {
                    let selected = self.state.config_files[0].clone();
                    self.state.selected_config = Some(selected);
                }
                Command::none()
            }
            Message::ConfigsLoaded(Err(e)) => {
                self.state.error = Some(format!("Error loading configs: {}", e));
                Command::none()
            }
            Message::ConfigSelected(config_path) => {
                self.state.selected_config = Some(config_path.clone());
                self.state.assignments = Vec::new();
                self.state.people = Vec::new();
                self.state.error = None;
                Command::perform(
                    utils::generate_schedule(config_path),
                    Message::ScheduleGenerated,
                )
            }
            Message::RefreshConfigList => {
                // Only refresh the file list, don't generate a schedule
                Command::perform(
                    async { crate::gui::find_config_files().await },
                    Message::ConfigsLoaded,
                )
            }
            Message::GenerateSchedule => {
                if let Some(config_path) = &self.state.selected_config {
                    Command::perform(
                        utils::generate_schedule(config_path.clone()),
                        Message::ScheduleGenerated,
                    )
                } else {
                    Command::none()
                }
            }
            Message::ScheduleGenerated(Ok(assignments)) => {
                // Store the assignments
                self.state.assignments = assignments.clone();
                self.state.selected_cell = None;

                // Generate people states from the config
                if let Some(config_path) = &self.state.selected_config
                    && let Ok(config) = load_config(config_path)
                {
                    let dates = get_weekdays(
                        &config.dates.from,
                        &config.dates.to,
                        &config.dates.weekdays,
                    );
                    let (_, people) = create_schedule(&dates, &config);
                    self.state.people = people;
                }

                Command::none()
            }
            Message::ScheduleGenerated(Err(e)) => {
                self.state.error = Some(format!("Error generating schedule: {}", e));
                Command::none()
            }
            Message::SaveScheduleWithDate => {
                if let Some(config_path) = self.state.selected_config.clone() {
                    if !self.state.assignments.is_empty() {
                        let filename = crate::gui::generate_filename(config_path);
                        Command::perform(async { Message::SaveSchedule(filename) }, |msg| msg)
                    } else {
                        self.state.error = Some("No schedule to save".to_string());
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }

            Message::SaveSchedule(filename) => {
                // Convert assignments to CSV for saving
                match assignments_to_csv(&self.state.assignments) {
                    Ok(csv_content) => {
                        // Create summary content directly from people states
                        let mut summary_content = String::new();
                        for person in &self.state.people {
                            let name = person.name();
                            let total = person.total_services();
                            summary_content.push_str(&format!("{}, total: {}", name, total));

                            for (day, count) in person.weekday_counts() {
                                summary_content.push_str(&format!(", {day}: {count}"));
                            }

                            summary_content.push_str(&format!(
                                ", different_place: {}\n",
                                person.different_place_services()
                            ));
                        }

                        // Save the file
                        let filename_for_message = filename.clone(); // Clone for the success message
                        Command::perform(
                            utils::save_file(filename, csv_content, summary_content),
                            move |result| {
                                if result.is_ok() {
                                    Message::ShowSuccessMessage(format!(
                                        "Schedule saved to {}",
                                        filename_for_message
                                    ))
                                } else {
                                    Message::ScheduleSaved(result)
                                }
                            },
                        )
                    }
                    Err(e) => {
                        self.state.error = Some(format!("Failed to create CSV: {}", e));
                        Command::none()
                    }
                }
            }
            Message::ScheduleSaved(Ok(())) => {
                // Successfully saved - this is now handled directly in the SaveSchedule handler
                Command::none()
            }

            Message::ShowSuccessMessage(message) => {
                self.state.success_message = Some(message);
                self.state.success_message_expires_at =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(3));

                // Schedule a check after 3 seconds
                Command::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        Message::CheckMessageExpiry
                    },
                    |msg| msg,
                )
            }

            Message::CheckMessageExpiry => {
                // Check if the success message has expired
                if let Some(expires_at) = self.state.success_message_expires_at
                    && std::time::Instant::now() >= expires_at
                {
                    self.state.success_message = None;
                    self.state.success_message_expires_at = None;
                }
                Command::none()
            }
            Message::ScheduleSaved(Err(e)) => {
                self.state.error = Some(format!("Error saving schedule: {}", e));
                Command::none()
            }
            Message::TabSelected(tab) => {
                self.state.active_tab = tab;
                Command::none()
            }
            Message::CellClicked(position) => self.state.handle_cell_click(position),
            Message::CellHovered(position) => {
                self.state.hovered_cell = Some(position);
                Command::none()
            }
            Message::MouseEntered(position) => {
                self.state.hovered_cell = Some(position);
                Command::none()
            }
            Message::MouseLeft => {
                self.state.hovered_cell = None;
                Command::none()
            }
            Message::Error(e) => {
                self.state.error = Some(e);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let title = text("Duty Roster").size(24);

        // Create config selector
        let config_selector = super::config::create_config_selector(
            &self.state.config_files,
            &self.state.selected_config,
            Message::ConfigSelected,
            Message::RefreshConfigList,
        );

        let generate_button =
            button(text("Generate Schedule").size(14)).on_press(Message::GenerateSchedule);
        let save_button = if !self.state.assignments.is_empty() {
            button(text("Save").size(14)).on_press(Message::SaveScheduleWithDate)
        } else {
            button(text("Save").size(14)).style(iced::theme::Button::Secondary)
        };

        let mut content = column![title, config_selector, row![generate_button, save_button]]
            .spacing(15)
            .padding(15);

        // Display error if any
        if let Some(error) = &self.state.error {
            content = content.push(
                text(format!("Error: {}", error))
                    .size(12)
                    .style(iced::Color::from_rgb(0.8, 0.0, 0.0)),
            );
        }

        // Display success message if any
        if let Some(message) = &self.state.success_message {
            content = content.push(
                text(message)
                    .size(12)
                    .style(iced::Color::from_rgb(0.0, 0.6, 0.0)),
            );
        }

        // Add tabs if content is available
        if !self.state.assignments.is_empty() || !self.state.people.is_empty() {
            // Create tab row
            let schedule_tab = button(
                text("Schedule")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .width(Length::FillPortion(1))
            .style(if self.state.active_tab == Tab::Schedule {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            })
            .on_press(Message::TabSelected(Tab::Schedule));

            let summary_tab = button(
                text("Summary")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .width(Length::FillPortion(1))
            .style(if self.state.active_tab == Tab::Summary {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            })
            .on_press(Message::TabSelected(Tab::Summary));

            content = content.push(row![schedule_tab, summary_tab].spacing(5));

            // Display content based on active tab
            match self.state.active_tab {
                Tab::Schedule => {
                    if !self.state.assignments.is_empty() {
                        let table_view = table::create_table_from_assignments(
                            &self.state.assignments,
                            self.state.selected_cell.as_ref(),
                            self.state.hovered_cell.as_ref(),
                        );
                        content =
                            content.push(scrollable(table_view).height(Length::FillPortion(3)));
                    }
                }
                Tab::Summary => {
                    if !self.state.people.is_empty() {
                        let summary_view =
                            summary::create_summary_view_from_people(&self.state.people);
                        content =
                            content.push(scrollable(summary_view).height(Length::FillPortion(3)));
                    }
                }
            }
        }

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::time::Instant;
    use crate::schedule::person_state::{PersonState, GroupState};
    use std::rc::Rc;
    use std::cell::RefCell;

    fn create_test_date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn create_test_app() -> DutyRosterApp {
        DutyRosterApp {
            state: AppState::new(),
        }
    }
    
    fn create_test_assignments() -> Vec<Assignment> {
        vec![
            Assignment {
                date: create_test_date(2025, 9, 1),
                place: "Place A".to_string(),
                person: "Person1".to_string(),
            },
            Assignment {
                date: create_test_date(2025, 9, 2),
                place: "Place B".to_string(),
                person: "Person2".to_string(),
            },
        ]
    }
    
    fn create_test_people() -> Vec<PersonState> {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        
        let mut person1 = PersonState::new(
            "Person1".to_string(),
            "Place A".to_string(),
            Rc::clone(&group_state),
        );
        
        let mut person2 = PersonState::new(
            "Person2".to_string(), 
            "Place B".to_string(),
            Rc::clone(&group_state),
        );
        
        person1.register_service(create_test_date(2025, 9, 1), "Place A".to_string());
        person2.register_service(create_test_date(2025, 9, 2), "Place B".to_string());
        
        vec![person1, person2]
    }

    #[test]
    fn test_new() {
        let (app, _command) = DutyRosterApp::new(());
        assert_eq!(app.state.config_files.len(), 0);
        assert_eq!(app.state.selected_config, None);
    }

    #[test]
    fn test_update_config_selected() {
        let mut app = create_test_app();
        
        // Test selecting a config
        let config_path = "test_config.toml".to_string();
        let message = Message::ConfigSelected(config_path.clone());
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.selected_config, Some(config_path));
    }

    #[test]
    fn test_update_tab_selected() {
        let mut app = create_test_app();
        
        // Test selecting a tab
        let message = Message::TabSelected(Tab::Summary);
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.active_tab, Tab::Summary);
    }

    #[test]
    fn test_update_cell_clicked() {
        let mut app = create_test_app();
        
        // Test clicking a cell
        let position = CellPosition { row: 1, column: 1 };
        let message = Message::CellClicked(position);
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.selected_cell, Some(position));
    }

    #[test]
    fn test_update_cell_hovered() {
        let mut app = create_test_app();
        
        // Test hovering over a cell
        let position = CellPosition { row: 1, column: 1 };
        let message = Message::CellHovered(position);
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.hovered_cell, Some(position));
    }

    #[test]
    fn test_update_mouse_left() {
        let mut app = create_test_app();
        
        // First set a hovered cell
        app.state.hovered_cell = Some(CellPosition { row: 1, column: 1 });
        
        // Test mouse leaving
        let message = Message::MouseLeft;
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.hovered_cell, None);
    }

    #[test]
    fn test_update_show_success_message() {
        let mut app = create_test_app();
        
        // Test showing a success message
        let message_text = "Success!".to_string();
        let message = Message::ShowSuccessMessage(message_text.clone());
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.success_message, Some(message_text));
        assert!(app.state.success_message_expires_at.is_some());
    }

    #[test]
    fn test_update_check_message_expiry() {
        let mut app = create_test_app();
        
        // Set a success message that has expired
        app.state.success_message = Some("Test message".to_string());
        app.state.success_message_expires_at = Some(Instant::now());
        
        // Let's make sure it's expired
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        // Test checking message expiry
        let message = Message::CheckMessageExpiry;
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.success_message, None);
        assert_eq!(app.state.success_message_expires_at, None);
    }

    #[test]
    fn test_update_configs_loaded() {
        let mut app = create_test_app();
        
        // Test loading configs
        let configs = vec!["config1.toml".to_string(), "config2.toml".to_string()];
        let message = Message::ConfigsLoaded(Ok(configs.clone()));
        
        let _cmd = app.update(message);
        
        assert_eq!(app.state.config_files, configs);
    }

    #[test]
    fn test_update_configs_loaded_error() {
        let mut app = create_test_app();
        
        // Test loading configs with error
        let error_message = "Failed to load configs".to_string();
        let message = Message::ConfigsLoaded(Err(error_message.clone()));
        
        let _cmd = app.update(message);
        
        // The app adds "Error loading configs: " prefix to the error message
        let expected_error = format!("Error loading configs: {}", error_message);
        assert_eq!(app.state.error, Some(expected_error));
    }

    #[test]
    fn test_view() {
        let app = create_test_app();
        
        // Test the view function
        let _element = app.view();
        
        // We can't easily test the actual UI rendering, but we can ensure the function runs without panicking
        assert_eq!(app.state.selected_config, None);
    }
    
    #[test]
    fn test_update_refresh_config_list() {
        let mut app = create_test_app();
        
        // Test refreshing config list
        let message = Message::RefreshConfigList;
        
        // This should return a Command
        let cmd = app.update(message);
        
        // We can't easily test the Command itself, but we can verify it's not Command::none()
        // Just ensure the test runs without panicking
        let _ = cmd;
    }
    
    #[test]
    fn test_update_generate_schedule_no_config() {
        let mut app = create_test_app();
        
        // Test generating a schedule without a selected config
        let message = Message::GenerateSchedule;
        
        // This should return Command::none()
        let cmd = app.update(message);
        
        // We can't easily test if it's Command::none(), so just ensure it runs without panicking
        let _ = cmd;
    }
    
    #[test]
    fn test_update_save_schedule_with_date_no_assignments() {
        let mut app = create_test_app();
        
        // Set a selected config but no assignments
        app.state.selected_config = Some("test_config.toml".to_string());
        
        // Test saving a schedule with date but no assignments
        let message = Message::SaveScheduleWithDate;
        
        let _cmd = app.update(message);
        
        // Verify an error was set
        assert_eq!(app.state.error, Some("No schedule to save".to_string()));
    }
    
    #[test]
    fn test_update_save_schedule_with_date_with_assignments() {
        let mut app = create_test_app();
        
        // Set a selected config and assignments
        app.state.selected_config = Some("test_config.toml".to_string());
        app.state.assignments = create_test_assignments();
        
        // Test saving a schedule with date
        let message = Message::SaveScheduleWithDate;
        
        // This should return a Command
        let cmd = app.update(message);
        
        // We can't easily test the Command itself, just ensure it runs without panicking
        let _ = cmd;
    }
    
    #[test]
    fn test_update_show_success_message_detailed() {
        let mut app = create_test_app();
        
        // Test showing a success message
        let message_text = "Success!".to_string();
        let message = Message::ShowSuccessMessage(message_text.clone());
        
        let _cmd = app.update(message);
        
        // Verify the success message was set
        assert_eq!(app.state.success_message, Some(message_text));
        assert!(app.state.success_message_expires_at.is_some());
    }
    
    #[test]
    fn test_update_schedule_saved_success() {
        let mut app = create_test_app();
        
        // Test handling a successful schedule save
        // Note: In the current implementation, this just returns Command::none()
        let message = Message::ScheduleSaved(Ok(()));
        
        let _cmd = app.update(message);
        
        // No assertions needed as this just returns Command::none()
    }
    
    #[test]
    fn test_update_schedule_generated_success() {
        let mut app = create_test_app();
        
        // Set up a selected config
        app.state.selected_config = Some("test_config.toml".to_string());
        
        // Create test assignments
        let date = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        let assignments = vec![
            Assignment {
                date,
                place: "Place A".to_string(),
                person: "Person1".to_string(),
            },
        ];
        
        // Test handling a successful schedule generation
        let message = Message::ScheduleGenerated(Ok(assignments.clone()));
        
        let _cmd = app.update(message);
        
        // Verify assignments were stored
        assert_eq!(app.state.assignments.len(), 1);
        assert_eq!(app.state.assignments[0].date, date);
        assert_eq!(app.state.assignments[0].place, "Place A");
        assert_eq!(app.state.assignments[0].person, "Person1");
        
        // Verify selected cell was reset
        assert_eq!(app.state.selected_cell, None);
    }
    
    #[test]
    fn test_update_schedule_saved_error() {
        let mut app = create_test_app();
        
        // Test handling a failed schedule save
        let error_message = "Failed to save schedule".to_string();
        let message = Message::ScheduleSaved(Err(error_message.clone()));
        
        let _cmd = app.update(message);
        
        // Verify the error was stored with the prefix
        assert_eq!(app.state.error, Some(format!("Error saving schedule: {}", error_message)));
    }
    
    #[test]
    fn test_update_schedule_generated_error() {
        let mut app = create_test_app();
        
        // Test handling a failed schedule generation
        let error_message = "Failed to generate schedule".to_string();
        let message = Message::ScheduleGenerated(Err(error_message.clone()));
        
        let _cmd = app.update(message);
        
        // Verify the error was stored with the prefix
        assert_eq!(app.state.error, Some(format!("Error generating schedule: {}", error_message)));
    }
    
    #[test]
    fn test_update_save_schedule() {
        let mut app = create_test_app();
        
        // Add test assignments
        let date = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        app.state.assignments = vec![
            Assignment {
                date,
                place: "Place A".to_string(),
                person: "Person1".to_string(),
            },
        ];
        
        // Add test people
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut person1 = PersonState::new(
            "Person1".to_string(),
            "Place A".to_string(),
            Rc::clone(&group_state),
        );
        person1.register_service(date, "Place A".to_string());
        app.state.people = vec![person1];
        
        // Test handling a save schedule message
        let filename = "test_schedule.csv".to_string();
        let message = Message::SaveSchedule(filename.clone());
        
        let cmd = app.update(message);
        
        // Verify a command was returned (we can't easily test the actual command)
        // Just check that it's not empty by using a dummy variable
        let _ = cmd;
        
        // The test passes if we get here without panicking
        assert!(true);
    }
}
