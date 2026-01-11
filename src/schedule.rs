//! core business logic, parse configuration and create the schedule

use std::cell::RefCell;
use std::rc::Rc;

use crate::config::{Config, Rule};
use chrono::NaiveDate;
use rand::rng;
use rand::seq::SliceRandom;

pub mod person_state;
pub use person_state::GroupState;
pub use person_state::PersonState;

pub(crate) fn create_people(config: &Config) -> Vec<PersonState> {
    let mut people: Vec<PersonState> = vec![];

    for group in &config.group {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        for member in &group.members {
            people.push(PersonState::new(
                format!("{} {}", member.name, group.name),
                group.place.clone(),
                Rc::clone(&group_state),
            ));
        }
    }

    people
}

/// Assignment captures a date, task(place) and person to do the job
#[derive(Debug, Clone)]
pub struct Assignment {
    pub date: NaiveDate,
    pub place: String,
    pub person: String,
}

/// parse the configuration and assign someone on the given dates for the defined tasks(places)
pub fn create_schedule(
    dates: &Vec<NaiveDate>,
    config: &Config,
) -> (Vec<Assignment>, Vec<PersonState>) {
    let mut people = create_people(config);

    let mut assignments = Vec::new();
    let filter_same_workid = config.rules.filter.contains(&Rule::FilterSamePlace);

    for date in dates {
        if config.dates.exceptions.contains(date) {
            continue;
        }

        let mut rng = rng();
        people.shuffle(&mut rng);

        for place_id in &config.places.places {
            let mut candidates: Vec<&mut PersonState> = people
                .iter_mut()
                .filter(|p| !filter_same_workid || &p.place() == place_id)
                .collect();

            // Sort by precomputed tuple keys
            candidates.sort_by_key(|p| p.sort_key(*date, place_id, &config.rules));

            if let Some(chosen) = candidates.first_mut() {
                assignments.push(Assignment {
                    date: *date,
                    place: place_id.clone(),
                    person: chosen.name(),
                });
                chosen.register_service(*date, place_id.clone());
            }
        }
    }

    (assignments, people)
}

#[cfg(test)]
mod tests {
    use crate::{config::load_config, dates::get_weekdays, schedule::create_schedule};
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn create_schedule_should_provide_reasonable_schedule() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/config.toml");
        let config = load_config(config_path.to_str().unwrap()).unwrap();
        let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);

        let (assignments, people) = create_schedule(&dates, &config);

        assert_eq!(
            assignments.len(),
            (dates.len() - config.dates.exceptions.len()) * config.places.places.len()
        );

        assert_eq!(
            assignments.len(),
            people.iter().map(|p| p.total_services()).sum()
        );

        assert_eq!(
            assignments.len(),
            people
                .iter()
                .map(|p| p.weekday_counts().values().copied().sum::<usize>())
                .sum()
        );
    }

    #[test]
    fn create_schedule_place_counts_should_match_assignments() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test/config.toml");
        let config = load_config(config_path.to_str().unwrap()).unwrap();
        let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);

        let (assignments, people) = create_schedule(&dates, &config);

        let mut expected: HashMap<(String, String), usize> = HashMap::new();
        for a in &assignments {
            *expected
                .entry((a.person.clone(), a.place.clone()))
                .or_default() += 1;
        }

        for person in &people {
            let name = person.name();
            let counts = person.place_counts();

            for place in &config.places.places {
                let got = counts.get(place).copied().unwrap_or(0);
                let exp = expected
                    .get(&(name.clone(), place.clone()))
                    .copied()
                    .unwrap_or(0);
                assert_eq!(got, exp, "mismatch for {name} at {place}");
            }
        }
    }
}
