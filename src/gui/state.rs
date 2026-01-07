use iced::Command;
use chrono::NaiveDate;
use std::collections::{BTreeMap, BTreeSet};

use crate::config::load_config;
use crate::dates::get_weekdays;
use crate::schedule::{create_schedule, Assignment, PersonState};

use super::{CellPosition, Message, Tab};
use super::assignment;

/// Application state
pub struct AppState {
    pub config_files: Vec<String>,
    pub selected_config: Option<String>,
    pub assignments: Vec<Assignment>,
    pub people: Vec<PersonState>,
    pub error: Option<String>,
    pub active_tab: Tab,
    pub selected_cell: Option<CellPosition>,
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            config_files: Vec::new(),
            selected_config: None,
            assignments: Vec::new(),
            people: Vec::new(),
            error: None,
            active_tab: Tab::Schedule,
            selected_cell: None,
        }
    }
    
    /// Handle a cell click
    pub fn handle_cell_click(&mut self, position: CellPosition) -> Command<Message> {
        // Don't allow selecting header row
        if position.row == 0 {
            return Command::none();
        }
        
        if let Some(prev_selected) = self.selected_cell.take() {
            // Second cell clicked - attempt to swap
            if prev_selected == position {
                // Clicked same cell twice - deselect
                Command::none()
            } else if position.row > 0 && prev_selected.row > 0 {
                // Get cell information
                let cell1_info = self.get_cell_info(prev_selected);
                let cell2_info = self.get_cell_info(position);
                
                if let (Some((date1, place1, person1)), Some((date2, place2, person2))) = (cell1_info, cell2_info) {
                    // Swap the assignments
                    if assignment::swap_assignments(
                        &mut self.assignments,
                        &mut self.people,
                        date1, &place1, &person1,
                        date2, &place2, &person2
                    ) {
                        // Regenerate the people states
                        if let Some(config_path) = &self.selected_config {
                            if let Ok(config) = load_config(config_path) {
                                let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);
                                let (_, people) = create_schedule(&dates, &config);
                                self.people = people;
                            }
                        }
                    }
                }
                
                Command::none()
            } else {
                Command::none()
            }
        } else {
            // First cell clicked - select it
            self.selected_cell = Some(position);
            Command::none()
        }
    }
    
    /// Get information about a cell at the given position
    pub fn get_cell_info(&self, pos: CellPosition) -> Option<(NaiveDate, String, String)> {
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
