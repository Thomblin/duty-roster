//! contains the configuration for the execution

use chrono::{NaiveDate, Weekday};
use serde::Deserialize;
use std::fs;

/// configuration root
#[derive(Deserialize, Debug)]
pub struct Config {
    pub dates: Dates,
    pub places: Places,
    pub group: Vec<Group>,
    pub rules: Rules,
}

/// the schedule can create tasks per day per place
#[derive(Deserialize, Debug)]
pub struct Places {
    pub places: Vec<String>,
}

/// date restrictions for schedule
#[derive(Deserialize, Debug)]
pub struct Dates {
    /// first day of schedule
    pub from: NaiveDate,
    /// last day of schedule
    pub to: NaiveDate,
    /// do not schedule work on these dates
    pub exceptions: Vec<NaiveDate>,
    /// schedule work only on these weekdays
    pub weekdays: Vec<Weekday>,
}

/// list of people to assign work to
/// several people can be assigned to a group
/// work for people within one group is spreat evenly across the calendar
#[derive(Deserialize, Debug)]
pub struct Group {
    /// name of group (for example family name or task force)
    pub name: String,
    /// group can be assigned to places where work needs to be done, compare struct Places
    pub place: String,
    /// list of members for this group
    pub members: Vec<Member>,
}

/// member of a group
#[derive(Deserialize, Debug, PartialEq)]
pub struct Member {
    /// name of this member
    pub name: String,
}

/// set of rules to apply when creating the schedule
#[derive(Deserialize, Debug)]
pub struct Rules {
    /// sort member by these rules to find best match for next task
    pub sort: Vec<Rule>,
    /// filter member according to these rules for next task
    pub filter: Vec<Rule>,
}

/// currently implemented rules
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Rule {
    FilterSamePlace,                 // assign people only to their own place
    SortByLeastServices,             // everyone works the same amount of hours
    SortByLessServicesAtSameWeekday, // everyone should work on each weekday the same amount
    SortByLastService,               // prefer people who were assigned further back in the past
    SortByMaxDistanceInGroup, // prefer people where a person of the same group worked the longest time ago
    SortByOwnPlace,           // prefer people within the same place
    SortByDifferentPlaceServices, // prefer people who were assigned to a different place less
}

/// load Config from a file
pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn test_load_config() {
        let config = load_config("test/config.toml").expect("Failed to load config");

        assert_eq!(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            config.dates.from
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            config.dates.to
        );
        assert_eq!(
            vec![
                NaiveDate::from_ymd_opt(2025, 2, 10).unwrap(),
                NaiveDate::from_ymd_opt(2025, 12, 24).unwrap(),
            ],
            config.dates.exceptions
        );
        assert_eq!(vec![Weekday::Mon, Weekday::Wed], config.dates.weekdays);

        assert_eq!(
            vec!["Place A".to_string(), "Place B".to_string()],
            config.places.places
        );

        assert_eq!(2, config.group.len());
        assert_eq!("Maier".to_string(), config.group[0].name,);
        assert_eq!("Place A".to_string(), config.group[0].place);
        assert_eq!(
            vec![
                Member {
                    name: "Alice".to_string()
                },
                Member {
                    name: "Bob".to_string()
                },
            ],
            config.group[0].members
        );
        assert_eq!("Doe".to_string(), config.group[1].name,);
        assert_eq!("Place B".to_string(), config.group[1].place);
        assert_eq!(
            vec![Member {
                name: "Charlie".to_string()
            },],
            config.group[1].members
        );
        assert_eq!(
            vec![
                Rule::SortByLeastServices,
                Rule::SortByLessServicesAtSameWeekday,
                Rule::SortByOwnPlace,
                Rule::SortByLastService,
                Rule::SortByMaxDistanceInGroup,
                Rule::SortByDifferentPlaceServices,
            ],
            config.rules.sort
        );
        assert_eq!(vec![Rule::FilterSamePlace], config.rules.filter);
    }
}
