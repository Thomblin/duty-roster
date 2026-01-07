use iced::widget::{button, column, container, pick_list, row, scrollable, text};
use iced::{executor, Application, Command, Element, Length, Settings, Theme};
use std::fs;
use std::path::Path;
use chrono::Local;

use crate::config::load_config;
use crate::csv::assignments_to_csv;
use crate::dates::get_weekdays;
use crate::schedule::create_schedule;

pub fn run() -> iced::Result {
    DutyRosterApp::run(Settings::default())
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Schedule,
    Summary,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigSelected(String),
    RefreshConfigList,
    GenerateSchedule,
    SaveSchedule(String, String, String), // filename, csv_content, summary
    SaveScheduleWithDate,
    ConfigsLoaded(Result<Vec<String>, String>),
    ScheduleGenerated(Result<(String, String), String>), // (csv_content, summary)
    ScheduleSaved(Result<(), String>),
    TabSelected(Tab),
    Error(String),
}

pub struct DutyRosterApp {
    config_files: Vec<String>,
    selected_config: Option<String>,
    csv_content: Option<String>,
    summary_content: Option<String>,
    error: Option<String>,
    active_tab: Tab,
}

impl Application for DutyRosterApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                config_files: Vec::new(),
                selected_config: None,
                csv_content: None,
                summary_content: None,
                error: None,
                active_tab: Tab::Schedule,
            },
            Command::perform(
                async { find_config_files().await },
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
                self.config_files = files;
                if !self.config_files.is_empty() && self.selected_config.is_none() {
                    let selected = self.config_files[0].clone();
                    self.selected_config = Some(selected);
                }
                Command::none()
            },
            Message::ConfigsLoaded(Err(e)) => {
                self.error = Some(format!("Error loading configs: {}", e));
                Command::none()
            },
            Message::ConfigSelected(config_path) => {
                self.selected_config = Some(config_path.clone());
                self.csv_content = None;
                self.error = None;
                Command::perform(
                    generate_schedule(config_path),
                    Message::ScheduleGenerated
                )
            },
            Message::RefreshConfigList => {
                // Only refresh the file list, don't generate a schedule
                Command::perform(
                    async { find_config_files().await },
                    Message::ConfigsLoaded
                )
            },
            Message::GenerateSchedule => {
                if let Some(config_path) = &self.selected_config {
                    Command::perform(
                        generate_schedule(config_path.clone()),
                        Message::ScheduleGenerated
                    )
                } else {
                    Command::none()
                }
            },
            Message::ScheduleGenerated(Ok((csv, summary))) => {
                self.csv_content = Some(csv);
                self.summary_content = Some(summary);
                Command::none()
            },
            Message::ScheduleGenerated(Err(e)) => {
                self.error = Some(format!("Error generating schedule: {}", e));
                Command::none()
            },
            Message::SaveScheduleWithDate => {
                if let (Some(config_path), Some(csv_content), Some(summary_content)) = (
                    self.selected_config.clone(),
                    self.csv_content.clone(),
                    self.summary_content.clone()
                ) {
                    // Create a date-stamped filename
                    let now = Local::now();
                    let date_stamp = now.format("%Y_%m_%d").to_string();
                    
                    let file_stem = Path::new(&config_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("schedule");
                        
                    let parent = Path::new(&config_path)
                        .parent()
                        .unwrap_or_else(|| Path::new(""));
                        
                    let out_path = parent.join(format!("{file_stem}_{date_stamp}.csv"));
                    let out_path_str = out_path.to_string_lossy().to_string();
                    
                    Command::perform(
                        async move {
                            Message::SaveSchedule(
                                out_path_str,
                                csv_content,
                                summary_content,
                            )
                        },
                        |msg| msg
                    )
                } else {
                    Command::none()
                }
            },
            
            Message::SaveSchedule(filename, csv_content, summary_content) => {
                Command::perform(
                    save_schedule(filename, csv_content, summary_content),
                    Message::ScheduleSaved
                )
            },
            Message::ScheduleSaved(Ok(())) => {
                // Successfully saved
                Command::none()
            },
            Message::ScheduleSaved(Err(e)) => {
                self.error = Some(format!("Error saving schedule: {}", e));
                Command::none()
            },
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Command::none()
            },
            Message::Error(error) => {
                self.error = Some(error);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let title = text("Duty Roster").size(24);

        let refresh_button = button(text("Refresh").size(14)).on_press(Message::RefreshConfigList);
        
        let config_dropdown = if self.config_files.is_empty() {
            row![text("No config files found").size(14), refresh_button]
        } else {
            row![
                text("Select config file:").size(14).width(Length::Fill),
                pick_list(
                    self.config_files.clone(),
                    self.selected_config.clone(),
                    Message::ConfigSelected
                )
                .width(Length::Fill),
                refresh_button
            ]
        };

        let generate_button = button(text("Generate Schedule").size(14)).on_press(Message::GenerateSchedule);
        let save_button = if self.csv_content.is_some() {
            button(text("Save").size(14)).on_press(Message::SaveScheduleWithDate)
        } else {
            button(text("Save").size(14)).style(iced::theme::Button::Secondary)
        };

        let mut content = column![title, config_dropdown, row![generate_button, save_button]]
            .spacing(15)
            .padding(15);

        // Display error if any
        if let Some(error) = &self.error {
            content = content.push(text(format!("Error: {}", error)).size(12).style(iced::Color::from_rgb(0.8, 0.0, 0.0)));
        }

        // Add tabs if content is available
        if self.csv_content.is_some() || self.summary_content.is_some() {
            // Create tab row
            let schedule_tab = button(
                text("Schedule").size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            )
            .width(Length::FillPortion(1))
            .style(if self.active_tab == Tab::Schedule {
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
            .style(if self.active_tab == Tab::Summary {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            })
            .on_press(Message::TabSelected(Tab::Summary));
            
            content = content.push(row![schedule_tab, summary_tab].spacing(5));
            
            // Display content based on active tab
            match self.active_tab {
                Tab::Schedule => {
                    if let Some(ref csv_content) = self.csv_content {
                        let table = create_table_from_csv(csv_content);
                        content = content.push(scrollable(table).height(Length::FillPortion(3)));
                    }
                },
                Tab::Summary => {
                    if let Some(ref summary) = self.summary_content {
                        let summary_view = create_summary_view(summary);
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

fn create_table_from_csv(csv_content: &str) -> Element<'_, Message> {
    let mut rows = Vec::new();
    
    for (i, line) in csv_content.lines().enumerate() {
        let cells: Vec<&str> = line.split(',').collect();
        let mut row_content = row![];
        
        for cell in cells {
            row_content = row_content.push(
                container(text(cell.trim_matches('"')).size(12))
                    .padding(3)
                    .width(Length::Fill)
            );
        }
        
        // Style header row differently
        if i == 0 {
            rows.push(container(row_content).style(|_: &Theme| {
                container::Appearance {
                    background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
                    ..Default::default()
                }
            }).into());
        } else {
            rows.push(container(row_content).into());
        }
    }
    
    column(rows).spacing(1).into()
}

async fn find_config_files() -> Result<Vec<String>, String> {
    // Look for config files in the current directory and subdirectories
    let mut config_files = Vec::new();
    
    // Start with the current directory
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                if let Some(path_str) = path.to_str() {
                    config_files.push(path_str.to_string());
                }
            }
        }
    }
    
    // Add test directory if it exists
    if let Ok(entries) = fs::read_dir("test") {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
                if let Some(path_str) = path.to_str() {
                    config_files.push(path_str.to_string());
                }
            }
        }
    }
    
    if config_files.is_empty() {
        return Err("No config files found".to_string());
    }
    
    Ok(config_files)
}

async fn generate_schedule(config_path: String) -> Result<(String, String), String> {
    match load_config(&config_path) {
        Ok(config) => {
            let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
            let (assignments, people) = create_schedule(&dates, &config);
            
            match assignments_to_csv(&assignments) {
                Ok(csv) => {
                    // Create summary content
                    let mut summary = String::new();
                    
                    for person in &people {
                        summary.push_str(&format!("{}, total: {}", person.name(), person.total_services()));
                        
                        for (day, count) in person.weekday_counts() {
                            summary.push_str(&format!(", {day}: {count}"));
                        }
                        
                        summary.push_str(&format!(", different_place: {}\n", person.different_place_services()));
                    }
                    
                    Ok((csv, summary))
                },
                Err(e) => Err(format!("Failed to create CSV: {}", e)),
            }
        },
        Err(e) => Err(format!("Failed to load config: {}", e)),
    }
}

async fn save_schedule(filename: String, csv_content: String, summary_content: String) -> Result<(), String> {
    use std::fs::File;
    use std::io::Write;
    
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

fn create_summary_view(summary: &str) -> Element<'_, Message> {
    let mut rows = Vec::new();
    
    // Header
    rows.push(
        container(
            text("Summary Information").size(14)
        )
        .padding(3)
        .style(|_: &Theme| {
            container::Appearance {
                background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
                ..Default::default()
            }
        })
        .into()
    );
    
    // Table header
    rows.push(
        container(
            row![
                text("Person").size(12).width(Length::FillPortion(2)),
                text("Total").size(12).width(Length::FillPortion(1)),
                text("Weekday Stats").size(12).width(Length::FillPortion(4)),
                text("Different Place").size(12).width(Length::FillPortion(1))
            ]
        )
        .padding(3)
        .style(|_: &Theme| {
            container::Appearance {
                background: Some(iced::Color::from_rgb(0.95, 0.95, 0.95).into()),
                ..Default::default()
            }
        })
        .into()
    );
    
    // Parse and format summary content as a table
    for line in summary.lines() {
        // Parse the line
        let parts: Vec<&str> = line.split(", ").collect();
        if parts.is_empty() {
            continue;
        }
        
        let person_name = parts[0];
        
        // Extract total services
        let total = parts.iter()
            .find(|part| part.starts_with("total: "))
            .map(|part| part.trim_start_matches("total: "))
            .unwrap_or("-");
        
        // Extract different place services
        let different_place = parts.iter()
            .find(|part| part.starts_with("different_place: "))
            .map(|part| part.trim_start_matches("different_place: "))
            .unwrap_or("-");
        
        // Extract weekday stats
        let weekday_stats = parts.iter()
            .filter(|part| {
                !part.starts_with("total: ") && 
                !part.starts_with("different_place: ") &&
                *part != &person_name
            })
            .map(|s| *s)
            .collect::<Vec<&str>>()
            .join(", ");
        
        rows.push(
            container(
                row![
                    text(person_name).size(12).width(Length::FillPortion(2)),
                    text(total).size(12).width(Length::FillPortion(1)),
                    text(weekday_stats).size(12).width(Length::FillPortion(4)),
                    text(different_place).size(12).width(Length::FillPortion(1))
                ]
            )
            .padding(3)
            .into()
        );
    }
    
    column(rows).spacing(1).into()
}
