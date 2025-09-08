//! helper function to store the generated assignments into a csv String

use chrono::NaiveDate;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
};

use crate::schedule::Assignment;

/// convert assignments to csv String
pub fn assignments_to_csv(assignments: &Vec<Assignment>) -> Result<String, Box<dyn Error>> {
    let mut places: BTreeSet<String> = BTreeSet::new();
    let mut data: BTreeMap<NaiveDate, BTreeMap<String, String>> = BTreeMap::new();

    for a in assignments {
        places.insert(a.place.clone());
        data.entry(a.date)
            .or_default()
            .insert(a.place.clone(), a.person.clone());
    }

    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b',')
        .quote_style(csv::QuoteStyle::Necessary)
        .quote(b'"')
        .double_quote(false)
        .escape(b'\\')
        .from_writer(vec![]);

    let mut header = vec![];
    header.push("date");
    for g in &places {
        header.push(g);
    }
    wtr.write_record(header)?;

    for (date, action) in data {
        let mut row = vec![];
        row.push(date.to_string());
        for g in &places {
            if let Some(person) = action.get(g) {
                row.push(person.clone());
            }
        }
        wtr.write_record(row)?;
    }

    Ok(String::from_utf8(wtr.into_inner()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_assignments_to_csv() {
        let assignments = vec![
            Assignment {
                date: NaiveDate::from_ymd_opt(2025, 9, 6).unwrap(),
                place: "PlaceA".to_string(),
                person: "Alice".to_string(),
            },
            Assignment {
                date: NaiveDate::from_ymd_opt(2025, 9, 6).unwrap(),
                place: "PlaceB".to_string(),
                person: "Bob".to_string(),
            },
            Assignment {
                date: NaiveDate::from_ymd_opt(2025, 9, 7).unwrap(),
                place: "PlaceA".to_string(),
                person: "Charlie".to_string(),
            },
            Assignment {
                date: NaiveDate::from_ymd_opt(2025, 9, 7).unwrap(),
                place: "PlaceB".to_string(),
                person: "Alice".to_string(),
            },
        ];

        let csv = assignments_to_csv(&assignments).unwrap();

        let expected = "\
date,PlaceA,PlaceB
2025-09-06,Alice,Bob
2025-09-07,Charlie,Alice
";

        assert_eq!(expected, csv);
    }
}
