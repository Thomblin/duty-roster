use iced::widget::{button, column, container, row, scrollable, text};
use iced::{executor, Application, Command, Element, Length, Theme};

use crate::config::load_config;
use crate::csv::assignments_to_csv;
use crate::dates::get_weekdays;
use crate::schedule::{create_schedule, Assignment};

use super::state::AppState;
use super::table;
use super::summary;
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
                Message::ConfigsLoaded
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
            },
            Message::ConfigsLoaded(Err(e)) => {
                self.state.error = Some(format!("Error loading configs: {}", e));
                Command::none()
            },
            Message::ConfigSelected(config_path) => {
                self.state.selected_config = Some(config_path.clone());
                self.state.assignments = Vec::new();
                self.state.people = Vec::new();
                self.state.error = None;
                Command::perform(
                    utils::generate_schedule(config_path),
                    Message::ScheduleGenerated
                )
            },
            Message::RefreshConfigList => {
                // Only refresh the file list, don't generate a schedule
                Command::perform(
                    async { crate::gui::find_config_files().await },
                    Message::ConfigsLoaded
                )
            },
            Message::GenerateSchedule => {
                if let Some(config_path) = &self.state.selected_config {
                    Command::perform(
                        utils::generate_schedule(config_path.clone()),
                        Message::ScheduleGenerated
                    )
                } else {
                    Command::none()
                }
            },
            Message::ScheduleGenerated(Ok(assignments)) => {
                // Store the assignments
                self.state.assignments = assignments.clone();
                self.state.selected_cell = None;
                
                // Generate people states from the config
                if let Some(config_path) = &self.state.selected_config {
                    if let Ok(config) = load_config(config_path) {
                        let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
                        let (_, people) = create_schedule(&dates, &config);
                        self.state.people = people;
                    }
                }
                
                Command::none()
            },
            Message::ScheduleGenerated(Err(e)) => {
                self.state.error = Some(format!("Error generating schedule: {}", e));
                Command::none()
            },
            Message::SaveScheduleWithDate => {
                if let Some(config_path) = self.state.selected_config.clone() {
                    if !self.state.assignments.is_empty() {
                        let filename = crate::gui::generate_filename(config_path);
                        Command::perform(
                            async { Message::SaveSchedule(filename) },
                            |msg| msg
                        )
                    } else {
                        self.state.error = Some("No schedule to save".to_string());
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            },
            
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
                            
                            summary_content.push_str(&format!(", different_place: {}\n", person.different_place_services()));
                        }
                        
                        // Save the file
                        let filename_for_message = filename.clone(); // Clone for the success message
                        Command::perform(
                            utils::save_file(filename, csv_content, summary_content),
                            move |result| {
                                if result.is_ok() {
                                    Message::ShowSuccessMessage(format!("Schedule saved to {}", filename_for_message))
                                } else {
                                    Message::ScheduleSaved(result)
                                }
                            }
                        )
                    },
                    Err(e) => {
                        self.state.error = Some(format!("Failed to create CSV: {}", e));
                        Command::none()
                    }
                }
            },
            Message::ScheduleSaved(Ok(())) => {
                // Successfully saved - this is now handled directly in the SaveSchedule handler
                Command::none()
            },
            
            Message::ShowSuccessMessage(message) => {
                self.state.success_message = Some(message);
                self.state.success_message_expires_at = Some(std::time::Instant::now() + std::time::Duration::from_secs(3));
                
                // Schedule a check after 3 seconds
                Command::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        Message::CheckMessageExpiry
                    },
                    |msg| msg
                )
            },
            
            Message::CheckMessageExpiry => {
                // Check if the success message has expired
                if let Some(expires_at) = self.state.success_message_expires_at {
                    if std::time::Instant::now() >= expires_at {
                        self.state.success_message = None;
                        self.state.success_message_expires_at = None;
                    }
                }
                Command::none()
            },
            Message::ScheduleSaved(Err(e)) => {
                self.state.error = Some(format!("Error saving schedule: {}", e));
                Command::none()
            },
            Message::TabSelected(tab) => {
                self.state.active_tab = tab;
                Command::none()
            },
            Message::CellClicked(position) => {
                self.state.handle_cell_click(position)
            },
            Message::CellHovered(position) => {
                self.state.hovered_cell = Some(position);
                Command::none()
            },
            Message::MouseEntered(position) => {
                self.state.hovered_cell = Some(position);
                Command::none()
            },
            Message::MouseLeft => {
                self.state.hovered_cell = None;
                Command::none()
            },
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
            Message::RefreshConfigList
        );

        let generate_button = button(text("Generate Schedule").size(14)).on_press(Message::GenerateSchedule);
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
            content = content.push(text(format!("Error: {}", error)).size(12).style(iced::Color::from_rgb(0.8, 0.0, 0.0)));
        }
        
        // Display success message if any
        if let Some(message) = &self.state.success_message {
            content = content.push(text(message).size(12).style(iced::Color::from_rgb(0.0, 0.6, 0.0)));
        }

        // Add tabs if content is available
        if !self.state.assignments.is_empty() || !self.state.people.is_empty() {
            // Create tab row
            let schedule_tab = button(
                text("Schedule").size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            )
            .width(Length::FillPortion(1))
            .style(if self.state.active_tab == Tab::Schedule {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            })
            .on_press(Message::TabSelected(Tab::Schedule));
            
            let summary_tab = button(
                text("Summary").size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
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
                            self.state.hovered_cell.as_ref()
                        );
                        content = content.push(scrollable(table_view).height(Length::FillPortion(3)));
                    }
                },
                Tab::Summary => {
                    if !self.state.people.is_empty() {
                        let summary_view = summary::create_summary_view_from_people(&self.state.people);
                        content = content.push(scrollable(summary_view).height(Length::FillPortion(3)));
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
