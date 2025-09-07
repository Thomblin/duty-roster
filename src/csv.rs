use chrono::NaiveDate;
use std::collections::{BTreeMap, BTreeSet};

use crate::schedule::Assignment;

pub fn assignments_to_csv(assignments: Vec<Assignment>) -> String {
    let mut places: BTreeSet<String> = BTreeSet::new();
    let mut data: BTreeMap<NaiveDate, BTreeMap<String, String>> = BTreeMap::new();

    for a in assignments {
        places.insert(a.place.clone());
        data.entry(a.date).or_default().insert(a.place, a.person);
    }

    let mut csv = String::new();
    // header
    csv.push_str("date");
    for g in &places {
        csv.push(',');
        csv.push_str(g);
    }
    csv.push('\n');

    // rows
    for (date, row) in data {
        csv.push_str(&date.to_string());
        for g in &places {
            csv.push(',');
            if let Some(person) = row.get(g) {
                csv.push_str(person);
            }
        }
        csv.push('\n');
    }

    csv
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
        ];

        let csv = assignments_to_csv(assignments);

        let expected = "\
date,PlaceA,PlaceB
2025-09-06,Alice,Bob
2025-09-07,Charlie,
";

        assert_eq!(csv, expected);
    }
}
