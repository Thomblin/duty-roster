//! some data helper functions

use chrono::Datelike;
use chrono::{Duration, NaiveDate, Weekday};

/// extract all weekdays within a gfiven timeframe
///
/// # Arguments
/// * `start` - first day to extract
/// * `end` - last day to extract
/// * `weekdays` - all weekdays to be extracted
///
pub fn get_weekdays(start: &NaiveDate, end: &NaiveDate, weekdays: &[Weekday]) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut current = *start;

    while current <= *end {
        if weekdays.contains(&current.weekday()) {
            dates.push(current);
        }
        current += Duration::days(1);
    }

    dates
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Weekday};

    use crate::dates::get_weekdays;

    #[test]
    fn returns_days_in_range() {
        let start = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 9, 15).unwrap();

        let result = get_weekdays(&start, &end, &[Weekday::Thu, Weekday::Fri]);

        let expected = vec![
            NaiveDate::from_ymd_opt(2025, 9, 4).unwrap(),
            NaiveDate::from_ymd_opt(2025, 9, 5).unwrap(),
            NaiveDate::from_ymd_opt(2025, 9, 11).unwrap(),
            NaiveDate::from_ymd_opt(2025, 9, 12).unwrap(),
        ];
        assert_eq!(expected, result);
    }
}
