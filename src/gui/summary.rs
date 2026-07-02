use std::collections::HashMap;

use iced::widget::{column, container, mouse_area, row, text};
use iced::{Element, FillPortion, Theme};

use super::Message;
use crate::schedule::{Assignment, PersonState};

// Container style functions
pub fn summary_header_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
        ..Default::default()
    }
}

pub fn summary_column_header_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgb(0.95, 0.95, 0.95).into()),
        ..Default::default()
    }
}

pub fn summary_person_highlight_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgb(0.85, 0.85, 0.85).into()),
        ..Default::default()
    }
}

pub fn summary_person_highlight_style_yellow(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgb(1.0, 1.0, 0.8).into()),
        ..Default::default()
    }
}

pub fn summary_person_highlight_style_green(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgb(0.8, 1.0, 0.8).into()),
        ..Default::default()
    }
}

pub fn summary_person_highlight_style_blue(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgb(0.8, 0.9, 1.0).into()),
        ..Default::default()
    }
}

fn highlight_slot_for_person(
    highlighted_names: &[Option<String>; 4],
    person_name: &str,
) -> Option<usize> {
    highlighted_names
        .iter()
        .position(|p| p.as_deref() == Some(person_name))
}

/// Build a map: base_person → HashMap<icon, count> from assignments that have extra tasks applied
pub(crate) fn extra_task_counts(
    assignments: &[Assignment],
) -> HashMap<String, HashMap<String, usize>> {
    let mut result: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for a in assignments {
        if a.person == a.base_person {
            continue;
        }
        // Icons are everything after base_person + space; guard in case of manual edits
        let Some(rest) = a.person.strip_prefix(a.base_person.as_str()) else {
            continue;
        };
        let suffix = rest.trim();
        for icon in suffix.split_whitespace() {
            *result
                .entry(a.base_person.clone())
                .or_default()
                .entry(icon.to_string())
                .or_default() += 1;
        }
    }
    result
}

/// Create a summary view from people states
pub fn create_summary_view_from_people<'a>(
    people: &'a [PersonState],
    assignments: &'a [Assignment],
    highlighted_names: &'a [Option<String>; 4],
) -> Element<'a, Message> {
    let extra_counts = extra_task_counts(assignments);
    let mut rows = Vec::new();

    // Header
    rows.push(
        container(text("Summary Information").size(14))
            .padding(3)
            .style(summary_header_style)
            .into(),
    );

    // Column headers
    rows.push(
        container(row![
            text("Person").size(12).width(FillPortion(2)),
            text("Total").size(12).width(FillPortion(1)),
            text("Weekday Stats").size(12).width(FillPortion(3)),
            text("Place Counts").size(12).width(FillPortion(3)),
            text("Different Place").size(12).width(FillPortion(1)),
            text("Extra Tasks").size(12).width(FillPortion(2)),
        ])
        .padding(3)
        .style(summary_column_header_style)
        .into(),
    );

    // Display each person's data directly from the PersonState objects
    for person in people {
        let person_name = format!("{} ({})", person.name(), person.place());
        let total = person.total_services().to_string();
        let different_place = person.different_place_services().to_string();
        let person_key = person.name();

        // Format weekday stats
        let weekday_stats = person
            .weekday_counts()
            .iter()
            .map(|(day, count)| format!("{day}: {count}"))
            .collect::<Vec<String>>()
            .join(", ");

        // Format place counts
        let place_stats = person
            .place_counts()
            .iter()
            .map(|(place, count)| format!("{place}: {count}"))
            .collect::<Vec<String>>()
            .join(", ");

        // Format extra task counts
        let extra_stats = if let Some(counts) = extra_counts.get(person.name().as_str()) {
            let mut entries: Vec<String> = counts
                .iter()
                .map(|(icon, n)| format!("{icon}: {n}"))
                .collect();
            entries.sort();
            entries.join(", ")
        } else {
            String::new()
        };

        let highlight_slot = highlight_slot_for_person(highlighted_names, person.name().as_str());

        let row_container = container(row![
            text(person_name).size(12).width(FillPortion(2)),
            text(total).size(12).width(FillPortion(1)),
            text(weekday_stats).size(12).width(FillPortion(2)),
            text(place_stats).size(12).width(FillPortion(3)),
            text(different_place).size(12).width(FillPortion(1)),
            text(extra_stats).size(12).width(FillPortion(2)),
        ])
        .padding(3);

        rows.push(match highlight_slot {
            Some(0) => mouse_area(row_container.style(summary_person_highlight_style))
                .on_press(Message::SummaryPersonClicked(person_key.clone()))
                .into(),
            Some(1) => mouse_area(row_container.style(summary_person_highlight_style_yellow))
                .on_press(Message::SummaryPersonClicked(person_key.clone()))
                .into(),
            Some(2) => mouse_area(row_container.style(summary_person_highlight_style_green))
                .on_press(Message::SummaryPersonClicked(person_key.clone()))
                .into(),
            Some(_) => mouse_area(row_container.style(summary_person_highlight_style_blue))
                .on_press(Message::SummaryPersonClicked(person_key.clone()))
                .into(),
            None => mouse_area(row_container)
                .on_press(Message::SummaryPersonClicked(person_key.clone()))
                .into(),
        });
    }

    column(rows).spacing(1).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::GroupState;
    use chrono::NaiveDate;
    use iced::Background;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn create_test_date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn create_test_people() -> Vec<PersonState> {
        let group_state = Rc::new(RefCell::new(GroupState::default()));

        // Create a person with some services
        let mut person1 = PersonState::new(
            "Person1".to_string(),
            "Place A".to_string(),
            Rc::clone(&group_state),
        );

        // Add some services on different days
        person1.register_service(create_test_date(2025, 9, 1), "Place A".to_string()); // Monday
        person1.register_service(create_test_date(2025, 9, 3), "Place B".to_string()); // Wednesday

        // Create another person with different services
        let mut person2 = PersonState::new(
            "Person2".to_string(),
            "Place B".to_string(),
            Rc::clone(&group_state),
        );

        person2.register_service(create_test_date(2025, 9, 2), "Place A".to_string()); // Tuesday

        vec![person1, person2]
    }

    #[test]
    fn test_summary_styles() {
        let theme = Theme::Light;
        let header = summary_header_style(&theme);
        assert!(header.background.is_some());

        let col_header = summary_column_header_style(&theme);
        assert!(col_header.background.is_some());

        let gray = summary_person_highlight_style(&theme);
        assert_eq!(
            gray.background,
            Some(Background::Color(iced::Color::from_rgb(0.85, 0.85, 0.85)))
        );

        let yellow = summary_person_highlight_style_yellow(&theme);
        assert_eq!(
            yellow.background,
            Some(Background::Color(iced::Color::from_rgb(1.0, 1.0, 0.8)))
        );

        let green = summary_person_highlight_style_green(&theme);
        assert_eq!(
            green.background,
            Some(Background::Color(iced::Color::from_rgb(0.8, 1.0, 0.8)))
        );

        let blue = summary_person_highlight_style_blue(&theme);
        assert_eq!(
            blue.background,
            Some(Background::Color(iced::Color::from_rgb(0.8, 0.9, 1.0)))
        );
    }

    #[test]
    fn test_highlight_slot_for_person_none() {
        let highlighted_names = [
            Some("Alice".to_string()),
            Some("Bob".to_string()),
            Some("Carol".to_string()),
            Some("Dave".to_string()),
        ];
        assert_eq!(highlight_slot_for_person(&highlighted_names, "Eve"), None);
    }

    #[test]
    fn test_highlight_slot_for_person_each_slot() {
        let highlighted_names = [
            Some("Alice".to_string()),
            Some("Bob".to_string()),
            Some("Carol".to_string()),
            Some("Dave".to_string()),
        ];

        assert_eq!(
            highlight_slot_for_person(&highlighted_names, "Alice"),
            Some(0)
        );
        assert_eq!(
            highlight_slot_for_person(&highlighted_names, "Bob"),
            Some(1)
        );
        assert_eq!(
            highlight_slot_for_person(&highlighted_names, "Carol"),
            Some(2)
        );
        assert_eq!(
            highlight_slot_for_person(&highlighted_names, "Dave"),
            Some(3)
        );
    }

    #[test]
    fn test_create_summary_view_from_people() {
        let people = create_test_people();

        // Create the summary view
        let element = create_summary_view_from_people(&people, &[], &[None, None, None, None]);

        // We can't easily test the actual UI rendering, but we can ensure the function runs without panicking
        // and returns an Element
        assert!(element.as_widget().children().len() > 0);
    }

    #[test]
    fn test_create_summary_view_from_empty_people() {
        // Test with empty people list
        let people: Vec<PersonState> = Vec::new();

        // Create the summary view
        let element = create_summary_view_from_people(&people, &[], &[None, None, None, None]);

        // Should still create headers even with no data
        assert!(element.as_widget().children().len() > 0);
    }

    #[test]
    fn test_summary_view_with_multiple_places() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut person = PersonState::new(
            "Alice".to_string(),
            "Place A".to_string(),
            Rc::clone(&group_state),
        );

        // Register services at multiple places
        person.register_service(create_test_date(2025, 9, 1), "Place A".to_string());
        person.register_service(create_test_date(2025, 9, 2), "Place B".to_string());
        person.register_service(create_test_date(2025, 9, 3), "Place C".to_string());
        person.register_service(create_test_date(2025, 9, 4), "Place A".to_string());

        let people = vec![person];
        let element = create_summary_view_from_people(&people, &[], &[None, None, None, None]);

        // Verify the element is created successfully
        assert!(element.as_widget().children().len() > 0);
    }

    #[test]
    fn test_summary_view_with_many_people() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut people = Vec::new();

        // Create 5 people with various service patterns
        for i in 0..5 {
            let mut person = PersonState::new(
                format!("Person{}", i),
                format!("Place{}", i),
                Rc::clone(&group_state),
            );

            // Each person has different number of services
            for j in 0..=i {
                person.register_service(
                    create_test_date(2025, 9, (j + 1) as u32),
                    format!("Place{}", j),
                );
            }

            people.push(person);
        }

        let element = create_summary_view_from_people(&people, &[], &[None, None, None, None]);

        // Should have created element successfully
        assert!(element.as_widget().children().len() > 0);
    }

    #[test]
    fn test_summary_view_verifies_place_counts() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut person = PersonState::new(
            "Test Person".to_string(),
            "Home".to_string(),
            Rc::clone(&group_state),
        );

        // Register multiple services to verify place_counts getter is called
        person.register_service(create_test_date(2025, 9, 1), "Home".to_string());
        person.register_service(create_test_date(2025, 9, 2), "Away".to_string());
        person.register_service(create_test_date(2025, 9, 3), "Home".to_string());

        // Verify place_counts is correctly populated
        let place_counts = person.place_counts();
        assert_eq!(*place_counts.get("Home").unwrap(), 2);
        assert_eq!(*place_counts.get("Away").unwrap(), 1);

        let people = vec![person];
        let element = create_summary_view_from_people(&people, &[], &[None, None, None, None]);

        // Verify element is created
        assert!(element.as_widget().children().len() > 0);
    }

    #[test]
    fn test_summary_view_with_special_characters_in_place_names() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut person = PersonState::new(
            "Alice".to_string(),
            "Place-A".to_string(),
            Rc::clone(&group_state),
        );

        // Test with special characters and spaces
        person.register_service(create_test_date(2025, 9, 1), "Place-A".to_string());
        person.register_service(create_test_date(2025, 9, 2), "Place B".to_string());
        person.register_service(create_test_date(2025, 9, 3), "Place_C".to_string());

        let people = vec![person];
        let element = create_summary_view_from_people(&people, &[], &[None, None, None, None]);

        assert!(element.as_widget().children().len() > 0);
    }
}
