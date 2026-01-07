use iced::widget::{column, container, row, text};
use iced::{Element, Length, Theme};

use super::Message;
use crate::schedule::PersonState;

/// Create a summary view from people states
pub fn create_summary_view_from_people(people: &[PersonState]) -> Element<'_, Message> {
    let mut rows = Vec::new();

    // Header
    rows.push(
        container(text("Summary Information").size(14))
            .padding(3)
            .style(|_: &Theme| container::Appearance {
                background: Some(iced::Color::from_rgb(0.9, 0.9, 0.9).into()),
                ..Default::default()
            })
            .into(),
    );

    // Column headers
    rows.push(
        container(row![
            text("Person").size(12).width(Length::FillPortion(2)),
            text("Total").size(12).width(Length::FillPortion(1)),
            text("Weekday Stats").size(12).width(Length::FillPortion(3)),
            text("Different Place")
                .size(12)
                .width(Length::FillPortion(1))
        ])
        .padding(3)
        .style(|_: &Theme| container::Appearance {
            background: Some(iced::Color::from_rgb(0.95, 0.95, 0.95).into()),
            ..Default::default()
        })
        .into(),
    );

    // Display each person's data directly from the PersonState objects
    for person in people {
        let person_name = person.name();
        let total = person.total_services().to_string();
        let different_place = person.different_place_services().to_string();

        // Format weekday stats
        let weekday_stats = person
            .weekday_counts()
            .iter()
            .map(|(day, count)| format!("{day}: {count}"))
            .collect::<Vec<String>>()
            .join(", ");

        rows.push(
            container(row![
                text(person_name).size(12).width(Length::FillPortion(2)),
                text(total).size(12).width(Length::FillPortion(1)),
                text(weekday_stats).size(12).width(Length::FillPortion(3)),
                text(different_place).size(12).width(Length::FillPortion(1))
            ])
            .padding(3)
            .into(),
        );
    }

    column(rows).spacing(1).into()
}
