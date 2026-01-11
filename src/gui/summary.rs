use iced::widget::{column, container, row, text};
use iced::{Element, Length, Theme};

use super::Message;
use crate::schedule::PersonState;

pub struct SummaryHeaderStyle;

impl container::StyleSheet for SummaryHeaderStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
            ..Default::default()
        }
    }
}

pub struct SummaryColumnHeaderStyle;

impl container::StyleSheet for SummaryColumnHeaderStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Color::from_rgb(0.95, 0.95, 0.95).into()),
            ..Default::default()
        }
    }
}

/// Create a summary view from people states
pub fn create_summary_view_from_people(people: &[PersonState]) -> Element<'_, Message> {
    let mut rows = Vec::new();

    // Header
    rows.push(
        container(text("Summary Information").size(14))
            .padding(3)
            .style(iced::theme::Container::Custom(Box::new(SummaryHeaderStyle)))
            .into(),
    );

    // Column headers
    rows.push(
        container(row![
            text("Person").size(12).width(Length::FillPortion(2)),
            text("Total").size(12).width(Length::FillPortion(1)),
            text("Weekday Stats").size(12).width(Length::FillPortion(3)),
            text("Place Counts").size(12).width(Length::FillPortion(3)),
            text("Different Place")
                .size(12)
                .width(Length::FillPortion(1))
        ])
        .padding(3)
        .style(iced::theme::Container::Custom(Box::new(
            SummaryColumnHeaderStyle,
        )))
        .into(),
    );

    // Display each person's data directly from the PersonState objects
    for person in people {
        let person_name = format!("{} ({})", person.name(), person.place());
        let total = person.total_services().to_string();
        let different_place = person.different_place_services().to_string();

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

        rows.push(
            container(row![
                text(person_name).size(12).width(Length::FillPortion(2)),
                text(total).size(12).width(Length::FillPortion(1)),
                text(weekday_stats).size(12).width(Length::FillPortion(2)),
                text(place_stats).size(12).width(Length::FillPortion(3)),
                text(different_place).size(12).width(Length::FillPortion(1))
            ])
            .padding(3)
            .into(),
        );
    }

    column(rows).spacing(1).into()
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
        let theme = Theme::default();
        let header = <SummaryHeaderStyle as container::StyleSheet>::appearance(
            &SummaryHeaderStyle,
            &theme,
        );
        assert!(header.background.is_some());

        let col_header = <SummaryColumnHeaderStyle as container::StyleSheet>::appearance(
            &SummaryColumnHeaderStyle,
            &theme,
        );
        assert!(col_header.background.is_some());
    }

    #[test]
    fn test_create_summary_view_from_people() {
        let people = create_test_people();

        // Create the summary view
        let element = create_summary_view_from_people(&people);

        // We can't easily test the actual UI rendering, but we can ensure the function runs without panicking
        // and returns an Element
        assert!(element.as_widget().children().len() > 0);
    }

    #[test]
    fn test_create_summary_view_from_empty_people() {
        // Test with empty people list
        let people: Vec<PersonState> = Vec::new();

        // Create the summary view
        let element = create_summary_view_from_people(&people);

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
        let element = create_summary_view_from_people(&people);

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

        let element = create_summary_view_from_people(&people);

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
        let element = create_summary_view_from_people(&people);

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
        let element = create_summary_view_from_people(&people);

        assert!(element.as_widget().children().len() > 0);
    }
}
