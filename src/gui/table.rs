use chrono::NaiveDate;
use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Element, Fill, Theme};
use std::collections::{BTreeMap, BTreeSet};

use super::{CellPosition, Message};
use crate::schedule::Assignment;

/// Represents the state of the schedule table
pub struct TableState {
    selected_cell: Option<CellPosition>,
    data: BTreeMap<NaiveDate, BTreeMap<String, String>>,
    dates: Vec<NaiveDate>,
    places: BTreeSet<String>,
}

// Helper to create a colored button style with active/hovered backgrounds
fn colored_button_style(
    active_color: iced::Color,
    hovered_color: iced::Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => hovered_color,
            _ => active_color,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: iced::Color::BLACK,
            border: iced::Border {
                radius: 2.0.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            ..Default::default()
        }
    }
}

// Style functions for button highlights
pub fn highlighted_cell_button_style_yellow(
    _theme: &Theme,
    status: button::Status,
) -> button::Style {
    colored_button_style(
        iced::Color::from_rgb(1.0, 1.0, 0.8),
        iced::Color::from_rgb(1.0, 1.0, 0.7),
    )(_theme, status)
}

pub fn highlighted_cell_button_style_green(
    _theme: &Theme,
    status: button::Status,
) -> button::Style {
    colored_button_style(
        iced::Color::from_rgb(0.8, 1.0, 0.8),
        iced::Color::from_rgb(0.75, 0.95, 0.75),
    )(_theme, status)
}

pub fn highlighted_cell_button_style_blue(_theme: &Theme, status: button::Status) -> button::Style {
    colored_button_style(
        iced::Color::from_rgb(0.8, 0.9, 1.0),
        iced::Color::from_rgb(0.75, 0.85, 0.95),
    )(_theme, status)
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
        let prev_selected = self.selected_cell;

        if prev_selected == Some(position) {
            // Clicked same cell twice - deselect
            self.selected_cell = None;
            prev_selected
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
        let date = match self.dates.get(pos.row - 1) {
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
        let person: String = match self
            .data
            .get(&date)
            .and_then(|row: &BTreeMap<String, String>| row.get(&place))
        {
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
    hovered_cell: Option<&'a CellPosition>,
    highlighted_names: &'a [Option<String>; 4],
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
            .width(Fill)
            .style(header_style),
    );

    // Add place column headers
    for place in &places {
        header_row = header_row.push(
            container(text(place.clone()).size(12))
                .padding(3)
                .width(Fill)
                .style(header_style),
        );
    }

    // Add the header row
    rows.push(container(header_row).style(header_style).into());

    // Create data rows
    for (row_idx, (date, assignments_for_date)) in data.iter().enumerate() {
        let mut row_content = row![];

        // Add date column
        let date_str: String = date.to_string();
        row_content = row_content.push(
            container(text(date_str).size(12))
                .padding(3)
                .width(Fill)
                .style(header_style),
        );

        // Add person cells for each place
        for (col_idx, place) in places.iter().enumerate() {
            let person: String = assignments_for_date.get(place).cloned().unwrap_or_default();

            // Create cell position for clickable cells
            let cell_position = CellPosition {
                row: row_idx + 1,    // +1 because row_idx starts at 0 but we have a header row
                column: col_idx + 1, // +1 because col_idx starts at 0 but we have a date column
            };

            // Check if this cell is selected or hovered
            let is_selected = selected_cell
                .map(|pos| pos.row == cell_position.row && pos.column == cell_position.column)
                .unwrap_or(false);

            let _is_hovered = hovered_cell
                .map(|pos| pos.row == cell_position.row && pos.column == cell_position.column)
                .unwrap_or(false);

            let highlight_slot = if person.is_empty() {
                None
            } else {
                highlighted_names
                    .iter()
                    .position(|p| p.as_deref() == Some(person.as_str()))
            };

            // Create clickable cell with appropriate style
            let cell_btn = if is_selected {
                button(text(person.clone()).size(12))
                    .width(Fill)
                    .padding(3)
                    .on_press(Message::CellClicked(cell_position))
                    .style(button::primary)
            } else if let Some(slot) = highlight_slot {
                button(text(person.clone()).size(12))
                    .width(Fill)
                    .padding(3)
                    .on_press(Message::CellClicked(cell_position))
                    .style(match slot {
                        0 => {
                            highlighted_cell_button_style_gray
                                as fn(&Theme, button::Status) -> button::Style
                        }
                        1 => highlighted_cell_button_style_yellow,
                        2 => highlighted_cell_button_style_green,
                        _ => highlighted_cell_button_style_blue,
                    })
            } else {
                button(text(person.clone()).size(12))
                    .width(Fill)
                    .padding(3)
                    .on_press(Message::CellClicked(cell_position))
                    .style(cell_button_style)
            };

            // Wrap in mouse_area to detect hover events
            let cell_with_hover = mouse_area(cell_btn)
                .on_enter(Message::CellHovered(cell_position))
                .on_right_press(Message::CellRightClicked(cell_position))
                .on_exit(Message::MouseLeft);

            row_content = row_content.push(cell_with_hover);
        }

        // Add the data row
        rows.push(container(row_content).into());
    }

    column(rows).spacing(1).into()
}

// Style function for header containers
pub fn header_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
        ..Default::default()
    }
}

// Style function for cell buttons (transparent bg, gray on hover)
pub fn cell_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    colored_button_style(
        iced::Color::TRANSPARENT,
        iced::Color::from_rgb(0.9, 0.9, 0.9),
    )(_theme, status)
}

// Style function for highlighted cells (light gray)
pub fn highlighted_cell_button_style_gray(_theme: &Theme, status: button::Status) -> button::Style {
    colored_button_style(
        iced::Color::from_rgb(0.85, 0.85, 0.85),
        iced::Color::from_rgb(0.8, 0.8, 0.8),
    )(_theme, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

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

    #[test]
    fn test_table_styles() {
        let theme = Theme::Light;

        let header = header_style(&theme);
        assert!(header.background.is_some());

        let active = cell_button_style(&theme, button::Status::Active);
        assert!(active.background.is_some());

        let hovered = cell_button_style(&theme, button::Status::Hovered);
        assert!(hovered.background.is_some());

        let active_hl = highlighted_cell_button_style_gray(&theme, button::Status::Active);
        assert!(active_hl.background.is_some());

        let hovered_hl = highlighted_cell_button_style_gray(&theme, button::Status::Hovered);
        assert!(hovered_hl.background.is_some());

        let active_y = highlighted_cell_button_style_yellow(&theme, button::Status::Active);
        assert!(active_y.background.is_some());

        let active_g = highlighted_cell_button_style_green(&theme, button::Status::Active);
        assert!(active_g.background.is_some());

        let active_b = highlighted_cell_button_style_blue(&theme, button::Status::Active);
        assert!(active_b.background.is_some());
    }

    #[test]
    fn test_table_state_new() {
        let assignments = create_test_assignments();
        let table_state = TableState::new(&assignments);

        // Check that the table state was created correctly
        assert_eq!(table_state.dates.len(), 2);
        assert_eq!(table_state.places.len(), 2);
        assert!(table_state.places.contains("Place A"));
        assert!(table_state.places.contains("Place B"));
        assert!(table_state.selected_cell.is_none());
    }

    #[test]
    fn test_table_state_select_cell_first_selection() {
        let assignments = create_test_assignments();
        let mut table_state = TableState::new(&assignments);

        // Test selecting a cell
        let pos = CellPosition { row: 1, column: 1 };
        let prev = table_state.select_cell(pos);

        assert!(prev.is_none());
        assert_eq!(table_state.selected_cell, Some(pos));
        assert_eq!(table_state.selected_cell(), Some(&pos));
    }

    #[test]
    fn test_table_state_deselect_cell() {
        // Create a simple TableState directly
        let mut table_state = TableState {
            selected_cell: None,
            data: BTreeMap::new(),
            dates: Vec::new(),
            places: BTreeSet::new(),
        };

        // Test selecting a cell
        let pos = CellPosition { row: 1, column: 1 };
        let prev = table_state.select_cell(pos);

        // Should return None (no previous selection) and set the selected cell
        assert!(prev.is_none());
        assert_eq!(table_state.selected_cell, Some(pos));

        // Test selecting the same cell again (should deselect)
        let prev = table_state.select_cell(pos);

        // Should return the previous selection and clear the selected cell
        assert_eq!(prev, Some(pos));
        assert!(table_state.selected_cell.is_none());
    }

    #[test]
    fn test_table_state_select_different_cell() {
        let assignments = create_test_assignments();
        let mut table_state = TableState::new(&assignments);

        // First select a cell
        let pos1 = CellPosition { row: 1, column: 1 };
        let _ = table_state.select_cell(pos1);

        // Then select a different cell
        let pos2 = CellPosition { row: 2, column: 2 };
        let prev = table_state.select_cell(pos2);

        assert_eq!(prev, Some(pos1));
        assert_eq!(table_state.selected_cell, Some(pos2));
    }

    #[test]
    fn test_get_cell_info() {
        let assignments = create_test_assignments();
        let table_state = TableState::new(&assignments);

        // Test getting valid cell info
        let cell_info = table_state.get_cell_info(CellPosition { row: 1, column: 1 });
        assert!(cell_info.is_some());

        if let Some((date, place, person)) = cell_info {
            assert_eq!(date, create_test_date(2025, 9, 1));
            assert_eq!(place, "Place A");
            assert_eq!(person, "Person1");
        }

        // Test getting invalid cell info (header row)
        let cell_info = table_state.get_cell_info(CellPosition { row: 0, column: 1 });
        assert!(cell_info.is_none());

        // Test getting invalid cell info (date column)
        let cell_info = table_state.get_cell_info(CellPosition { row: 1, column: 0 });
        assert!(cell_info.is_none());

        // Test getting invalid cell info (out of bounds)
        let cell_info = table_state.get_cell_info(CellPosition {
            row: 10,
            column: 10,
        });
        assert!(cell_info.is_none());
    }

    #[test]
    fn test_create_table_from_assignments() {
        let assignments = create_test_assignments();
        let highlighted_names = [None, None, None, None];

        // Create a table with no selection or hover
        let element = create_table_from_assignments(&assignments, None, None, &highlighted_names);

        // We can't easily test the actual UI rendering, but we can ensure the function runs without panicking
        // and returns an Element
        assert!(element.as_widget().children().len() > 0);

        // Create a table with selection
        let selected_cell = Some(CellPosition { row: 1, column: 1 });
        let element = create_table_from_assignments(
            &assignments,
            selected_cell.as_ref(),
            None,
            &highlighted_names,
        );
        assert!(element.as_widget().children().len() > 0);

        // Create a table with hover
        let hovered_cell = Some(CellPosition { row: 1, column: 1 });
        let element = create_table_from_assignments(
            &assignments,
            None,
            hovered_cell.as_ref(),
            &highlighted_names,
        );
        assert!(element.as_widget().children().len() > 0);

        let highlighted_names = [Some("Person1".to_string()), None, None, None];
        let element = create_table_from_assignments(&assignments, None, None, &highlighted_names);
        assert!(element.as_widget().children().len() > 0);
    }
}
