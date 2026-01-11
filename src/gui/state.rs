use chrono::NaiveDate;
use iced::Command;
use std::collections::{BTreeMap, BTreeSet};

use crate::schedule::{Assignment, PersonState};

use super::assignment;
use super::{CellPosition, Message, Tab};

/// Application state
pub struct AppState {
    pub config_files: Vec<String>,
    pub selected_config: Option<String>,
    pub assignments: Vec<Assignment>,
    pub people: Vec<PersonState>,
    pub error: Option<String>,
    pub success_message: Option<String>,
    pub success_message_expires_at: Option<std::time::Instant>,
    pub active_tab: Tab,
    pub selected_cell: Option<CellPosition>,
    pub hovered_cell: Option<CellPosition>,
    pub highlighted_names: [Option<String>; 4],
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config_files: Vec::new(),
            selected_config: None,
            assignments: Vec::new(),
            people: Vec::new(),
            error: None,
            success_message: None,
            success_message_expires_at: None,
            active_tab: Tab::Schedule,
            selected_cell: None,
            hovered_cell: None,
            highlighted_names: [None, None, None, None],
        }
    }
}

impl AppState {
    /// Create a new empty state
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_highlighted_name(&mut self, person: String) {
        if let Some(slot) = self
            .highlighted_names
            .iter()
            .position(|p| p.as_deref() == Some(&person))
        {
            self.highlighted_names[slot] = None;
            return;
        }

        if let Some(slot) = self.highlighted_names.iter().position(|p| p.is_none()) {
            self.highlighted_names[slot] = Some(person);
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

                if let (Some((date1, place1, person1)), Some((date2, place2, person2))) =
                    (cell1_info, cell2_info)
                {
                    // Swap the assignments and update person statistics
                    assignment::swap_assignments(
                        &mut self.assignments,
                        &mut self.people,
                        date1,
                        &place1,
                        &person1,
                        date2,
                        &place2,
                        &person2,
                    );
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
        let date = match dates.get(pos.row - 1) {
            // -1 because row 0 is header
            Some(date) => *date,
            None => return None,
        };

        // Get the place from the column index
        let place: String = match places_vec.get(pos.column - 1) {
            // -1 because column 0 is date
            Some(place) => place.clone(),
            None => return None,
        };

        // Get the person from the assignments
        let person: String = match data
            .get(&date)
            .and_then(|row: &BTreeMap<String, String>| row.get(&place))
        {
            Some(person) => person.clone(),
            None => return None,
        };

        Some((date, place, person))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::GroupState;
    use chrono::NaiveDate;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn create_test_date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
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

    #[allow(dead_code)]
    fn create_test_people() -> Vec<PersonState> {
        let group_state1 = Rc::new(RefCell::new(GroupState::default()));
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));

        let mut person1 = PersonState::new(
            "Person1".to_string(),
            "Place A".to_string(),
            Rc::clone(&group_state1),
        );

        let mut person2 = PersonState::new(
            "Person2".to_string(),
            "Place B".to_string(),
            Rc::clone(&group_state2),
        );

        // Register initial services
        person1.register_service(create_test_date(2025, 9, 1), "Place A".to_string());
        person2.register_service(create_test_date(2025, 9, 2), "Place B".to_string());

        vec![person1, person2]
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.config_files.is_empty());
        assert!(state.selected_config.is_none());
        assert!(state.assignments.is_empty());
        assert!(state.people.is_empty());
        assert!(state.error.is_none());
        assert!(state.success_message.is_none());
        assert!(state.success_message_expires_at.is_none());
        assert_eq!(state.active_tab, Tab::Schedule);
        assert!(state.selected_cell.is_none());
        assert!(state.hovered_cell.is_none());
        assert_eq!(state.highlighted_names, [None, None, None, None]);
    }

    #[test]
    fn test_get_cell_info_valid_position() {
        let mut state = AppState::default();
        state.assignments = create_test_assignments();

        // Test valid position
        let cell_info = state.get_cell_info(CellPosition { row: 1, column: 1 });
        assert!(cell_info.is_some());

        if let Some((date, place, person)) = cell_info {
            assert_eq!(date, create_test_date(2025, 9, 1));
            assert_eq!(place, "Place A");
            assert_eq!(person, "Person1");
        }
    }

    #[test]
    fn test_get_cell_info_invalid_position() {
        let mut state = AppState::default();
        state.assignments = create_test_assignments();

        // Test header row (row 0)
        let cell_info = state.get_cell_info(CellPosition { row: 0, column: 1 });
        assert!(cell_info.is_none());

        // Test date column (column 0)
        let cell_info = state.get_cell_info(CellPosition { row: 1, column: 0 });
        assert!(cell_info.is_none());

        // Test out of bounds
        let cell_info = state.get_cell_info(CellPosition {
            row: 10,
            column: 10,
        });
        assert!(cell_info.is_none());
    }

    #[test]
    fn test_handle_cell_click_first_selection() {
        let mut state = AppState::default();
        state.assignments = create_test_assignments();

        // First click should select the cell
        let pos = CellPosition { row: 1, column: 1 };
        let _ = state.handle_cell_click(pos);

        assert_eq!(state.selected_cell, Some(pos));
    }

    #[test]
    fn test_handle_cell_click_deselect() {
        let mut state = AppState::default();
        state.assignments = create_test_assignments();

        // First click selects the cell
        let pos = CellPosition { row: 1, column: 1 };
        let _ = state.handle_cell_click(pos);

        // Second click on same cell deselects it
        let _ = state.handle_cell_click(pos);

        assert_eq!(state.selected_cell, None);
    }

    #[test]
    fn test_handle_cell_click_header_row() {
        let mut state = AppState::default();
        state.assignments = create_test_assignments();

        // Clicking on header row should do nothing
        let pos = CellPosition { row: 0, column: 1 };
        let _ = state.handle_cell_click(pos);

        // Should not be selected
        assert_eq!(state.selected_cell, None);
    }

    #[test]
    fn test_handle_cell_click_swap() {
        let mut state = AppState::default();
        // Use a single date with two places so we can click two valid cells on the same row.
        let date = create_test_date(2025, 9, 1);
        state.assignments = vec![
            Assignment {
                date,
                place: "Place A".to_string(),
                person: "Person1".to_string(),
            },
            Assignment {
                date,
                place: "Place B".to_string(),
                person: "Person2".to_string(),
            },
        ];

        // People must have matching services registered, otherwise swapping won't update stats.
        let group_state1 = Rc::new(RefCell::new(GroupState::default()));
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));

        let mut person1 = PersonState::new(
            "Person1".to_string(),
            "Place A".to_string(),
            Rc::clone(&group_state1),
        );
        person1.register_service(date, "Place A".to_string());

        let mut person2 = PersonState::new(
            "Person2".to_string(),
            "Place B".to_string(),
            Rc::clone(&group_state2),
        );
        person2.register_service(date, "Place B".to_string());

        state.people = vec![person1, person2];

        // First click selects the cell
        let pos1 = CellPosition { row: 1, column: 1 };
        let _ = state.handle_cell_click(pos1);

        // Second click on different cell attempts swap
        let pos2 = CellPosition { row: 1, column: 2 };
        let _ = state.handle_cell_click(pos2);

        // After swap attempt, no cell should be selected
        assert_eq!(state.selected_cell, None);

        // Verify that the swap happened.
        let a = state
            .assignments
            .iter()
            .find(|a| a.date == date && a.place == "Place A")
            .unwrap();
        let b = state
            .assignments
            .iter()
            .find(|a| a.date == date && a.place == "Place B")
            .unwrap();
        assert_eq!(a.person, "Person2");
        assert_eq!(b.person, "Person1");
    }
}
