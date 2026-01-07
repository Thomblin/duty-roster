use iced::widget::{button, column, container, pick_list, row, scrollable, text};
use iced::{executor, Application, Command, Element, Length, Settings, Theme};
use std::fs;
use std::path::Path;
use std::collections::{BTreeMap, BTreeSet};
use chrono::{Local, NaiveDate};

use crate::config::{load_config, Config};
use crate::csv::assignments_to_csv;
use crate::dates::get_weekdays;
use crate::schedule::{create_schedule, Assignment, PersonState};

pub fn run() -> iced::Result {
    DutyRosterApp::run(Settings::default())
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Schedule,
    Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellPosition {
    row: usize,
    column: usize,
}

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
    Error(String),
}

pub struct DutyRosterApp {
    config_files: Vec<String>,
    selected_config: Option<String>,
    assignments: Vec<Assignment>,
    people: Vec<PersonState>,
    config: Option<Config>,
    error: Option<String>,
    active_tab: Tab,
    selected_cell: Option<CellPosition>,
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
                assignments: Vec::new(),
                people: Vec::new(),
                config: None,
                error: None,
                active_tab: Tab::Schedule,
                selected_cell: None,
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
                self.assignments = Vec::new();
                self.people = Vec::new();
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
            Message::ScheduleGenerated(Ok(assignments)) => {
                // Store the assignments
                self.assignments = assignments.clone();
                self.selected_cell = None;
                
                // Generate people states from the config
                if let Some(config_path) = &self.selected_config {
                    if let Ok(config) = load_config(config_path) {
                        let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
                        let (_, people) = create_schedule(&dates, &config);
                        self.people = people;
                    }
                }
                
                Command::none()
            },
            Message::ScheduleGenerated(Err(e)) => {
                self.error = Some(format!("Error generating schedule: {}", e));
                Command::none()
            },
            Message::SaveScheduleWithDate => {
                if let Some(config_path) = self.selected_config.clone() {
                    if !self.assignments.is_empty() {
                        let filename = generate_filename(config_path);
                        Command::perform(
                            async { Message::SaveSchedule(filename) },
                            |msg| msg
                        )
                    } else {
                        self.error = Some("No schedule to save".to_string());
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            },
            
            Message::SaveSchedule(filename) => {
                // Convert assignments to CSV for saving
                match assignments_to_csv(&self.assignments) {
                    Ok(csv_content) => {
                        // Create summary content directly from people states
                        let mut summary_content = String::new();
                        for person in &self.people {
                            summary_content.push_str(&format!("{}, total: {}", person.name(), person.total_services()));
                            
                            for (day, count) in person.weekday_counts() {
                                summary_content.push_str(&format!(", {day}: {count}"));
                            }
                            
                            summary_content.push_str(&format!(", different_place: {}\n", person.different_place_services()));
                        }
                        
                        // Save the file
                        Command::perform(
                            save_file(filename, csv_content, summary_content),
                            Message::ScheduleSaved
                        )
                    },
                    Err(e) => {
                        self.error = Some(format!("Failed to create CSV: {}", e));
                        Command::none()
                    }
                }
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
            Message::CellClicked(position) => {
                // Handle cell selection and swapping
                if position.row == 0 {
                    // Don't allow selecting header row
                    Command::none()
                } else if let Some(prev_selected) = self.selected_cell.take() {
                    // Second cell clicked - attempt to swap
                    if prev_selected == position {
                        // Clicked same cell twice - deselect
                        Command::none()
                    } else if position.row > 0 && prev_selected.row > 0 {
                        // Swap the assignments
                        if self.swap_assignments(prev_selected, position) {
                            // Regenerate the people states
                            if let Some(config_path) = &self.selected_config {
                                if let Ok(config) = load_config(config_path) {
                                    let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
                                    let (_, people) = create_schedule(&dates, &config);
                                    self.people = people;
                                }
                            }
                            Command::none()
                        } else {
                            Command::none()
                        }
                    } else {
                        Command::none()
                    }
                } else {
                    // First cell clicked - select it
                    self.selected_cell = Some(position);
                    Command::none()
                }
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
        let save_button = if !self.assignments.is_empty() {
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
        if !self.assignments.is_empty() || !self.people.is_empty() {
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
                    if !self.assignments.is_empty() {
                        let table = create_table_from_assignments(&self.assignments, self.selected_cell.as_ref());
                        content = content.push(scrollable(table).height(Length::FillPortion(3)));
                    }
                },
                Tab::Summary => {
                    if !self.people.is_empty() {
                        let summary_view = create_summary_view_from_people(&self.people);
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

fn create_table_from_assignments<'a>(assignments: &'a [Assignment], selected_cell: Option<&'a CellPosition>) -> Element<'a, Message> {
    let mut rows = Vec::new();
    
    // First, organize assignments by date and place
    let mut places: BTreeSet<String> = BTreeSet::new();
    let mut data: BTreeMap<NaiveDate, BTreeMap<String, String>> = BTreeMap::new();
    
    // Extract all places and organize data by date and place
    for assignment in assignments {
        places.insert(assignment.place.clone());
        data.entry(assignment.date)
            .or_default()
            .insert(assignment.place.clone(), assignment.person.clone());
    }
    
    // Create header row with places
    let mut header_row = row![];
    
    // Add date column header
    header_row = header_row.push(
        container(text("date").size(12))
            .padding(3)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(HeaderStyle)))
    );
    
    // Add place column headers
    for place in &places {
        header_row = header_row.push(
            container(text(place).size(12))
                .padding(3)
                .width(Length::Fill)
                .style(iced::theme::Container::Custom(Box::new(HeaderStyle)))
        );
    }
    
    // Add the header row
    rows.push(
        container(header_row)
            .style(iced::theme::Container::Custom(Box::new(HeaderRowStyle)))
            .into()
    );
    
    // Create data rows
    for (row_idx, (date, assignments_for_date)) in data.iter().enumerate() {
        let mut row_content = row![];
        
        // Add date column
        let date_str: String = date.to_string();
        row_content = row_content.push(
            container(text(&date_str).size(12))
                .padding(3)
                .width(Length::Fill)
                .style(iced::theme::Container::Custom(Box::new(HeaderStyle)))
        );
        
        // Add person cells for each place
        for (col_idx, place) in places.iter().enumerate() {
            let person: String = assignments_for_date.get(place).cloned().unwrap_or_default();
            
            // Create cell position for clickable cells
            let cell_position = CellPosition { 
                row: row_idx + 1, // +1 because row_idx starts at 0 but we have a header row
                column: col_idx + 1 // +1 because col_idx starts at 0 but we have a date column
            };
            
            // Check if this cell is selected
            let is_selected = selected_cell
                .map(|pos| pos.row == cell_position.row && pos.column == cell_position.column)
                .unwrap_or(false);
            
            // Create clickable cell
            let btn_style = if is_selected {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Text
            };
            
            let cell_btn = button(text(&person).size(12))
                .width(Length::Fill)
                .padding(3)
                .on_press(Message::CellClicked(cell_position))
                .style(btn_style);
            
            row_content = row_content.push(cell_btn);
        }
        
        // Add the data row
        rows.push(container(row_content).into());
    }
    
    column(rows).spacing(1).into()
}

// Custom style for header cells
struct HeaderStyle;

impl container::StyleSheet for HeaderStyle {
    type Style = Theme;
    
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Color::from_rgb(0.95, 0.95, 0.95).into()),
            ..Default::default()
        }
    }
}

// Custom style for header row
struct HeaderRowStyle;

impl container::StyleSheet for HeaderRowStyle {
    type Style = Theme;
    
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
            ..Default::default()
        }
    }
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

async fn generate_schedule(config_path: String) -> Result<Vec<Assignment>, String> {
    match load_config(&config_path) {
        Ok(config) => {
            let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
            let (assignments, _) = create_schedule(&dates, &config);
            Ok(assignments)
        },
        Err(e) => Err(format!("Failed to load config: {}", e)),
    }
}

fn generate_filename(config_path: String) -> String {
    let path = Path::new(&config_path);
    let file_stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("schedule");
    
    let date_stamp = Local::now().format("%Y_%m_%d").to_string();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    
    let out_path = parent.join(format!("{file_stem}_{date_stamp}.csv"));
    out_path.to_string_lossy().to_string()
}

async fn save_file(filename: String, csv_content: String, summary_content: String) -> Result<(), String> {
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

impl DutyRosterApp {
    // Swap assignments between two cells
    fn swap_assignments(&mut self, pos1: CellPosition, pos2: CellPosition) -> bool {
        // Convert positions to dates and places
        let (date1, place1, person1) = match self.get_assignment_info(pos1) {
            Some(info) => info,
            None => return false,
        };
        
        let (date2, place2, person2) = match self.get_assignment_info(pos2) {
            Some(info) => info,
            None => return false,
        };
        
        // Find and update the assignments
        let mut found1 = false;
        let mut found2 = false;
        
        for assignment in &mut self.assignments {
            if assignment.date == date1 && assignment.place == place1 {
                assignment.person = person2.clone();
                found1 = true;
            } else if assignment.date == date2 && assignment.place == place2 {
                assignment.person = person1.clone();
                found2 = true;
            }
            
            if found1 && found2 {
                break;
            }
        }
        
        // Update the people's service records
        // In a real implementation, we would need to update the PersonState objects
        // to reflect the changes in assignments, but that would require more complex logic
        // to remove and re-register services
        
        found1 && found2
    }
    
    // Get assignment information from a cell position
    fn get_assignment_info(&self, pos: CellPosition) -> Option<(NaiveDate, String, String)> {
        // Ignore header row and date column
        if pos.row == 0 || pos.column == 0 {
            return None;
        }
        
        // First, organize assignments by date and place
        let mut places: BTreeSet<String> = BTreeSet::new();
        let mut dates: Vec<NaiveDate> = Vec::new();
        let mut data: BTreeMap<NaiveDate, BTreeMap<String, String>> = BTreeMap::new();
        
        // Extract all places and organize data by date and place
        for assignment in &self.assignments {
            places.insert(assignment.place.clone());
            dates.push(assignment.date);
            data.entry(assignment.date)
                .or_default()
                .insert(assignment.place.clone(), assignment.person.clone());
        }
        
        // Sort dates
        dates.sort();
        dates.dedup();
        
        // Convert places to a vector for indexing
        let places_vec: Vec<String> = places.into_iter().collect();
        
        // Check if position is valid
        if pos.row > dates.len() || pos.column > places_vec.len() {
            return None;
        }
        
        // Get the date from the row index
        let date = match dates.get(pos.row - 1) { // -1 because row 0 is header
            Some(date) => *date,
            None => return None,
        };
        
        // Get the place from the column index
        let place: String = match places_vec.get(pos.column - 1) { // -1 because column 0 is date
            Some(place) => place.clone(),
            None => return None,
        };
        
        // Get the person from the assignments
        let person: String = match data.get(&date).and_then(|row: &BTreeMap<String, String>| row.get(&place)) {
            Some(person) => person.clone(),
            None => return None,
        };
        
        Some((date, place, person))
    }
}


fn create_summary_view_from_people(people: &[PersonState]) -> Element<'_, Message> {
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
    
    // Column headers
    rows.push(
        container(
            row![
                text("Person").size(12).width(Length::FillPortion(2)),
                text("Total").size(12).width(Length::FillPortion(1)),
                text("Weekday Stats").size(12).width(Length::FillPortion(3)),
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
    
    // Display each person's data directly from the PersonState objects
    for person in people {
        let person_name = person.name();
        let total = person.total_services().to_string();
        let different_place = person.different_place_services().to_string();
        
        // Format weekday stats
        let weekday_stats = person.weekday_counts()
            .iter()
            .map(|(day, count)| format!("{day}: {count}"))
            .collect::<Vec<String>>()
            .join(", ");
        
        rows.push(
            container(
                row![
                    text(person_name).size(12).width(Length::FillPortion(2)),
                    text(total).size(12).width(Length::FillPortion(1)),
                    text(weekday_stats).size(12).width(Length::FillPortion(3)),
                    text(different_place).size(12).width(Length::FillPortion(1))
                ]
            )
            .padding(3)
            .into()
        );
    }
    
    column(rows).spacing(1).into()
}
