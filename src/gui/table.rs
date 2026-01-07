use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Theme};
use std::collections::{BTreeMap, BTreeSet};
use chrono::NaiveDate;

use crate::schedule::Assignment;
use super::{CellPosition, Message};

/// Represents the state of the schedule table
pub struct TableState {
    selected_cell: Option<CellPosition>,
    data: BTreeMap<NaiveDate, BTreeMap<String, String>>,
    dates: Vec<NaiveDate>,
    places: BTreeSet<String>,
}

impl TableState {
    /// Create a new TableState from assignments
    pub fn new(assignments: &[Assignment]) -> Self {
        let mut places = BTreeSet::new();
        let mut dates = Vec::new();
        let mut data: BTreeMap<NaiveDate, BTreeMap<String, String>> = BTreeMap::new();
        
        // Extract all places and organize data by date and place
        for assignment in assignments {
            places.insert(assignment.place.clone());
            dates.push(assignment.date);
            data.entry(assignment.date)
                .or_default()
                .insert(assignment.place.clone(), assignment.person.clone());
        }
        
        // Sort dates
        dates.sort();
        dates.dedup();
        
        Self {
            selected_cell: None,
            data,
            dates,
            places,
        }
    }
    
    /// Select a cell in the table
    pub fn select_cell(&mut self, position: CellPosition) -> Option<CellPosition> {
        let prev_selected = self.selected_cell.take();
        
        if prev_selected == Some(position) {
            // Clicked same cell twice - deselect
            None
        } else {
            // Select new cell
            self.selected_cell = Some(position);
            prev_selected
        }
    }
    
    /// Get information about a cell at the given position
    pub fn get_cell_info(&self, pos: CellPosition) -> Option<(NaiveDate, String, String)> {
        // Ignore header row and date column
        if pos.row == 0 || pos.column == 0 {
            return None;
        }
        
        // Convert places to a vector for indexing
        let places_vec: Vec<String> = self.places.iter().cloned().collect();
        
        // Check if position is valid
        if pos.row > self.dates.len() || pos.column > places_vec.len() {
            return None;
        }
        
        // Get the date from the row index
        let date = match self.dates.get(pos.row - 1) { // -1 because row 0 is header
            Some(date) => *date,
            None => return None,
        };
        
        // Get the place from the column index
        let place: String = match places_vec.get(pos.column - 1) { // -1 because column 0 is date
            Some(place) => place.clone(),
            None => return None,
        };
        
        // Get the person from the assignments
        let person: String = match self.data.get(&date).and_then(|row: &BTreeMap<String, String>| row.get(&place)) {
            Some(person) => person.clone(),
            None => return None,
        };
        
        Some((date, place, person))
    }
    
    /// Get the selected cell
    pub fn selected_cell(&self) -> Option<&CellPosition> {
        self.selected_cell.as_ref()
    }
}

/// Create a table view from assignments
pub fn create_table_from_assignments<'a>(
    assignments: &'a [Assignment], 
    selected_cell: Option<&'a CellPosition>,
    hovered_cell: Option<&'a CellPosition>
) -> Element<'a, Message> {
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
            
            // Check if this cell is selected or hovered
            let is_selected = selected_cell
                .map(|pos| pos.row == cell_position.row && pos.column == cell_position.column)
                .unwrap_or(false);
                
            let _is_hovered = hovered_cell
                .map(|pos| pos.row == cell_position.row && pos.column == cell_position.column)
                .unwrap_or(false);
            
            // Create clickable cell
            let btn_style = if is_selected {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Custom(Box::new(CellButtonStyle))
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
pub struct HeaderStyle;

impl container::StyleSheet for HeaderStyle {
    type Style = Theme;
    
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
            ..Default::default()
        }
    }
}

// Custom style for header row
pub struct HeaderRowStyle;

impl container::StyleSheet for HeaderRowStyle {
    type Style = Theme;
    
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
            ..Default::default()
        }
    }
}

// Custom style for cells
pub struct CellButtonStyle;

impl button::StyleSheet for CellButtonStyle {
    type Style = Theme;
    
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Color::TRANSPARENT.into()),
            text_color: iced::Color::BLACK,
            border: iced::Border {
                radius: 2.0.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            shadow_offset: iced::Vector::default(),
            ..Default::default()
        }
    }
    
    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
            text_color: iced::Color::BLACK,
            border: iced::Border {
                radius: 2.0.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            shadow_offset: iced::Vector::default(),
            ..Default::default()
        }
    }
}
