use chrono::Datelike;
use chrono::NaiveDate;
use chrono::Weekday;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::config::Rule;
use crate::config::Rules;

#[derive(Debug, Default)]
pub struct GroupState {
    last_service: Option<NaiveDate>, // last date served in this group
}

#[derive(Clone, Debug)]
pub struct PersonState {
    name: String,
    place: String,

    // tracking
    total_services: usize,
    last_service: Option<NaiveDate>,
    weekday_counts: HashMap<Weekday, usize>, // weekday → count
    place_counts: HashMap<String, usize>,    // place → count
    group_state: Rc<RefCell<GroupState>>,
    different_place_services: usize,
}

impl PersonState {
    pub fn new(name: String, place: String, group_state: Rc<RefCell<GroupState>>) -> Self {
        Self {
            name,
            place,
            total_services: 0,
            last_service: None,
            weekday_counts: HashMap::new(),
            place_counts: HashMap::new(),
            group_state,
            different_place_services: 0,
        }
    }

    pub fn register_service(&mut self, date: NaiveDate, place: String) {
        self.total_services += 1;
        self.last_service = Some(date);
        *self.weekday_counts.entry(date.weekday()).or_default() += 1;
        *self.place_counts.entry(place.clone()).or_default() += 1;
        *self.group_state.borrow_mut() = GroupState {
            last_service: Some(date),
        };
        if place != self.place {
            self.different_place_services += 1;
        }
    }

    /// Unregister a service for this person
    ///
    /// This is used when swapping assignments between people
    pub fn unregister_service(&mut self, date: NaiveDate, place: String) {
        if self.total_services > 0 {
            self.total_services -= 1;
        }

        // Update weekday counts
        if let Some(count) = self.weekday_counts.get_mut(&date.weekday())
            && *count > 0
        {
            *count -= 1;
            if *count == 0 {
                self.weekday_counts.remove(&date.weekday());
            }
        }

        // Update place counts
        if let Some(count) = self.place_counts.get_mut(&place)
            && *count > 0
        {
            *count -= 1;
            if *count == 0 {
                self.place_counts.remove(&place);
            }
        }

        // Update different place services
        if place != self.place && self.different_place_services > 0 {
            self.different_place_services -= 1;
        }

        // Update last service date if needed
        if self.last_service == Some(date) {
            // Find the next most recent service date
            // For simplicity, we'll just set it to None
            // In a more complete implementation, we would track all service dates
            self.last_service = None;
        }

        // Note: We don't update the group_state here because that would affect other people
        // in the same group. In a real implementation, we might want to recalculate the
        // group's last service date based on all members.
    }

    /// Convert a person into a sortable key tuple according to rules
    pub(super) fn sort_key(&self, date: NaiveDate, place_id: &str, rules: &Rules) -> Vec<i64> {
        rules
            .sort
            .iter()
            .map(|rule| {
                match rule {
                    Rule::SortByLeastServices => self.total_services as i64,
                    Rule::SortByOwnPlace => {
                        if self.place == place_id { 0 } else { 1 } // smaller = preferred
                    }
                    Rule::SortByDifferentPlaceServices => {
                        if self.place != place_id {
                            self.different_place_services as i64
                        } else {
                            i64::MIN / self.different_place_services.max(1) as i64
                        }
                    }
                    Rule::SortByLastService => {
                        match self.last_service {
                            Some(d) => (d.num_days_from_ce() / 7) as i64, // earlier last service is smaller, however calculate only on a weekly basis to not overule rules like SortByDifferentPlaceServices
                            None => i64::MIN,
                        }
                    }
                    Rule::SortByLessServicesAtSameWeekday => {
                        *self.weekday_counts.get(&date.weekday()).unwrap_or(&0) as i64
                    }
                    Rule::SortByMaxDistanceInGroup => {
                        match self.group_state.borrow().last_service {
                            Some(d) => d.num_days_from_ce() as i64, // earlier last service is smaller
                            None => i64::MIN,
                        }
                    }
                    Rule::FilterSamePlace => 0,
                }
            })
            .collect()
    }

    pub fn total_services(&self) -> usize {
        self.total_services
    }

    pub fn weekday_counts(&self) -> HashMap<Weekday, usize> {
        self.weekday_counts.clone()
    }

    pub fn place_counts(&self) -> HashMap<String, usize> {
        self.place_counts.clone()
    }

    pub fn different_place_services(&self) -> usize {
        self.different_place_services
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn place(&self) -> String {
        self.place.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn new_person_has_clean_state() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let p = PersonState::new(
            "Alice".to_string(),
            "A".to_string(),
            Rc::clone(&group_state),
        );
        assert_eq!(p.name, "Alice");
        assert_eq!(p.place, "A");
        assert_eq!(p.total_services, 0);
        assert!(p.last_service.is_none());
        assert!(p.group_state.borrow().last_service.is_none());
        assert!(p.weekday_counts.is_empty());
    }

    #[test]
    fn register_service_updates_counters() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Bob".to_string(), "B".to_string(), Rc::clone(&group_state));
        let date = d(2023, 9, 6); // Wednesday
        p.register_service(date, "B".to_string());

        assert_eq!(p.total_services, 1);
        assert_eq!(p.last_service, Some(date));
        assert_eq!(p.group_state.borrow().last_service, Some(date));
        assert_eq!(*p.weekday_counts.get(&Weekday::Wed).unwrap(), 1);
        assert_eq!(p.different_place_services, 0);
    }

    #[test]
    fn register_service_updates_group_state_for_all_people() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut bob = PersonState::new("Bob".to_string(), "B".to_string(), Rc::clone(&group_state));
        let alex = PersonState::new("Alex".to_string(), "B".to_string(), Rc::clone(&group_state));

        let date = d(2023, 9, 6); // Wednesday
        bob.register_service(date, "B".to_string());

        assert_eq!(alex.group_state.borrow().last_service, Some(date));
    }

    #[test]
    fn register_service_increments_weekday_counts() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Bob".to_string(), "B".to_string(), Rc::clone(&group_state));
        let wed1 = d(2023, 9, 6);
        let wed2 = d(2023, 9, 13);

        p.register_service(wed1, "B".to_string());
        p.register_service(wed2, "B".to_string());

        assert_eq!(*p.weekday_counts.get(&Weekday::Wed).unwrap(), 2);
    }

    #[test]
    fn register_service_increments_different_place_services() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Bob".to_string(), "B".to_string(), Rc::clone(&group_state));
        let wed1 = d(2023, 9, 6);
        let wed2 = d(2023, 9, 13);

        p.register_service(wed1, "C".to_string());
        p.register_service(wed2, "C".to_string());

        assert_eq!(p.different_place_services, 2);
    }

    #[test]
    fn register_service_increments_different_place_services2() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Bob".to_string(), "B".to_string(), Rc::clone(&group_state));
        let wed1 = d(2023, 9, 6);
        let wed2 = d(2023, 9, 13);

        p.register_service(wed1, "C".to_string());
        p.register_service(wed2, "B".to_string());

        assert_eq!(p.different_place_services, 1);
    }

    #[test]
    fn unregister_service_decrements_total_services() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new(
            "Alice".to_string(),
            "A".to_string(),
            Rc::clone(&group_state),
        );
        let date = d(2023, 9, 6);

        p.register_service(date, "A".to_string());
        assert_eq!(p.total_services, 1);

        p.unregister_service(date, "A".to_string());
        assert_eq!(p.total_services, 0);
    }

    #[test]
    fn unregister_service_decrements_weekday_counts() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new(
            "Alice".to_string(),
            "A".to_string(),
            Rc::clone(&group_state),
        );
        let wed = d(2023, 9, 6); // Wednesday

        p.register_service(wed, "A".to_string());
        assert_eq!(*p.weekday_counts.get(&Weekday::Wed).unwrap(), 1);

        p.unregister_service(wed, "A".to_string());
        assert!(!p.weekday_counts.contains_key(&Weekday::Wed));
    }

    #[test]
    fn unregister_service_decrements_different_place_services() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new(
            "Alice".to_string(),
            "A".to_string(),
            Rc::clone(&group_state),
        );
        let date = d(2023, 9, 6);

        p.register_service(date, "B".to_string()); // Different place
        assert_eq!(p.different_place_services, 1);

        p.unregister_service(date, "B".to_string());
        assert_eq!(p.different_place_services, 0);
    }

    #[test]
    fn unregister_service_updates_last_service() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new(
            "Alice".to_string(),
            "A".to_string(),
            Rc::clone(&group_state),
        );
        let date = d(2023, 9, 6);

        p.register_service(date, "A".to_string());
        assert_eq!(p.last_service, Some(date));

        p.unregister_service(date, "A".to_string());
        assert_eq!(p.last_service, None);
    }

    #[test]
    fn sort_key_least_services() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new(
            "Charlie".to_string(),
            "C".to_string(),
            Rc::clone(&group_state),
        );
        let date = d(2023, 9, 6);

        // no services yet
        let rules = Rules {
            filter: vec![],
            sort: vec![Rule::SortByLeastServices],
        };
        assert_eq!(p.sort_key(date, "C", &rules), vec![0]);

        // after one service
        p.register_service(date, "C".to_string());
        assert_eq!(p.sort_key(date, "C", &rules), vec![1]);

        // after two services
        p.register_service(date, "C".to_string());
        assert_eq!(p.sort_key(date, "C", &rules), vec![2]);
    }

    #[test]
    fn sort_key_own_place_preferred() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let p = PersonState::new("Dana".to_string(), "X".to_string(), Rc::clone(&group_state));
        let rules = Rules {
            filter: vec![],
            sort: vec![Rule::SortByOwnPlace],
        };
        let date = d(2023, 9, 6);

        assert_eq!(p.sort_key(date, "X", &rules), vec![0]);
        assert_eq!(p.sort_key(date, "Y", &rules), vec![1]);
    }

    #[test]
    fn sort_key_last_service_earlier_is_smaller() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Eve".to_string(), "Z".to_string(), Rc::clone(&group_state));
        let rules = Rules {
            filter: vec![],
            sort: vec![Rule::SortByLastService],
        };
        let date1 = d(2023, 9, 1);
        let date2 = d(2023, 9, 10);

        p.register_service(date1, "Z".to_string());
        let key1 = p.sort_key(date2, "Z", &rules)[0];

        p.register_service(date2, "Z".to_string());
        let key2 = p.sort_key(date2, "Z", &rules)[0];

        assert!(key1 < key2); // earlier service gives smaller value
    }

    #[test]
    fn sort_key_max_distance_in_group_prefers_longer_gap() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new(
            "Frank".to_string(),
            "F".to_string(),
            Rc::clone(&group_state),
        );
        let rules = Rules {
            filter: vec![],
            sort: vec![Rule::SortByMaxDistanceInGroup],
        };
        let start = d(2023, 1, 1);

        let key0 = p.sort_key(start, "F", &rules)[0];

        let work1 = d(2022, 12, 13);
        p.register_service(work1, "F".to_string());
        let key1 = p.sort_key(start, "F", &rules)[0];

        assert!(key0 < key1);

        let work2 = d(2022, 12, 15);
        p.register_service(work2, "F".to_string());
        let key2 = p.sort_key(start, "F", &rules)[0];

        assert!(key1 < key2);
    }

    #[test]
    fn sort_key_max_distance_in_group_prefers_person_without_group() {
        let rules = Rules {
            filter: vec![],
            sort: vec![Rule::SortByMaxDistanceInGroup],
        };

        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut stefan = PersonState::new(
            "Frank".to_string(),
            "F".to_string(),
            Rc::clone(&group_state),
        );
        let martina = PersonState::new(
            "Martina".to_string(),
            "F".to_string(),
            Rc::clone(&group_state),
        );
        stefan.register_service(d(2025, 9, 4), "F".to_string());
        let key_stefan = stefan.sort_key(d(2025, 9, 11), "F", &rules)[0];

        let key_martina = martina.sort_key(d(2025, 9, 11), "F", &rules)[0];

        let group_state2 = Rc::new(RefCell::new(GroupState::default()));
        let petraq = PersonState::new(
            "Petraq".to_string(),
            "F".to_string(),
            Rc::clone(&group_state2),
        );
        let key_petraq = petraq.sort_key(d(2025, 9, 11), "F", &rules)[0];

        assert!(key_petraq < key_stefan);
        assert!(key_petraq < key_martina);
    }

    #[test]
    fn sort_key_with_multiple_rules() {
        let date = d(2023, 9, 10); // Sunday

        // Person A: 1 service in own place
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut a = PersonState::new(
            "Alice".to_string(),
            "G".to_string(),
            Rc::clone(&group_state),
        );
        a.register_service(d(2023, 9, 1), "G".to_string());

        // Person B: 2 services, not in own place
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));
        let mut b = PersonState::new("Bob".to_string(), "H".to_string(), Rc::clone(&group_state2));
        b.register_service(d(2023, 9, 2), "H".to_string());
        b.register_service(d(2023, 9, 3), "H".to_string());

        // Rules: fewest services → own place → longest distance at place
        let rules = Rules {
            sort: vec![Rule::SortByLeastServices, Rule::SortByOwnPlace],
            filter: vec![],
        };

        let key_a = a.sort_key(date, "G", &rules);
        let key_b = b.sort_key(date, "G", &rules);

        // Alice has fewer services, is at place, and has place distance
        // Bob has more services, not at place
        assert!(key_a < key_b, "Alice should sort before Bob");
    }

    #[test]
    fn sort_key_with_multiple_rules_place_first() {
        let date = d(2023, 9, 10); // Sunday

        // Person A: 1 service in own place
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut a = PersonState::new(
            "Alice".to_string(),
            "G".to_string(),
            Rc::clone(&group_state),
        );
        a.register_service(d(2023, 9, 1), "G".to_string());

        // Person B: 2 services, not in own place
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));
        let mut b = PersonState::new("Bob".to_string(), "H".to_string(), Rc::clone(&group_state2));
        b.register_service(d(2023, 9, 2), "H".to_string());
        b.register_service(d(2023, 9, 3), "H".to_string());

        // Rules: own place → longest distance at place → fewest services (last tie-breaker)
        let rules = Rules {
            sort: vec![Rule::SortByOwnPlace, Rule::SortByLeastServices],
            filter: vec![],
        };

        let key_a = a.sort_key(date, "H", &rules);
        let key_b = b.sort_key(date, "H", &rules);

        // Alice is at place, Bob isn’t → Alice should win immediately
        assert!(key_b < key_a, "Bob should sort before Alice");
    }

    #[test]
    fn sort_key_reverse_case_tiebreaking() {
        let date = d(2023, 9, 10);

        // Xavier: at place, but very recent place service
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut x = PersonState::new(
            "Xavier".to_string(),
            "G".to_string(),
            Rc::clone(&group_state),
        );
        x.register_service(d(2023, 9, 9), "G".to_string()); // just 1 day ago

        // Yara: not at place, but longer service distance
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));
        let mut y = PersonState::new(
            "Yara".to_string(),
            "H".to_string(),
            Rc::clone(&group_state2),
        );
        y.register_service(d(2023, 8, 1), "H".to_string()); // long ago

        // Rules: own place → max distance at place → fewest services
        let rules = Rules {
            sort: vec![Rule::SortByOwnPlace, Rule::SortByLastService],
            filter: vec![],
        };

        let key_x = x.sort_key(date, "G", &rules);
        let key_y = y.sort_key(date, "G", &rules);

        // Even though Y has better place distance, X is in the right place
        // Since SortByOwnPlace comes first, Xavier must win
        assert!(
            key_x < key_y,
            "Xavier should sort before Yara due to place match"
        );

        // Now flip priority: put distance before own place
        let flipped_rules = Rules {
            sort: vec![Rule::SortByLastService, Rule::SortByOwnPlace],
            filter: vec![],
        };

        let key_x2 = x.sort_key(date, "G", &flipped_rules);
        let key_y2 = y.sort_key(date, "G", &flipped_rules);

        // Now Yara should win because distance is more important than place
        assert!(
            key_y2 < key_x2,
            "Yara should sort before Xavier due to longer distance"
        );
    }

    #[test]
    fn sort_by_different_place_services() {
        let date = d(2023, 9, 10); // Sunday

        // Person A: 1 service in own place
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut a = PersonState::new(
            "Alice".to_string(),
            "G".to_string(),
            Rc::clone(&group_state),
        );
        a.register_service(d(2023, 9, 1), "G".to_string());

        // Person B: 2 services, not in own place
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));
        let mut b = PersonState::new("Bob".to_string(), "H".to_string(), Rc::clone(&group_state2));
        b.register_service(d(2023, 9, 1), "G".to_string());

        // Rules: fewest services → own place → longest distance at place
        let rules = Rules {
            sort: vec![Rule::SortByDifferentPlaceServices],
            filter: vec![],
        };

        let key_a = a.sort_key(date, "I", &rules);
        let key_b = b.sort_key(date, "I", &rules);

        // Alice has fewer services in a different place, prefer Bob
        assert!(key_a < key_b, "Alice should sort before Bob");
    }

    #[test]
    fn sort_by_different_place_services2() {
        let date = d(2023, 9, 10); // Sunday

        // Person A: 1 service in own place
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut a = PersonState::new(
            "Alice".to_string(),
            "G".to_string(),
            Rc::clone(&group_state),
        );
        a.register_service(d(2023, 9, 1), "G".to_string());

        // Person B: 2 services, not in own place
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));
        let mut b = PersonState::new("Bob".to_string(), "H".to_string(), Rc::clone(&group_state2));
        b.register_service(d(2023, 9, 1), "G".to_string());

        // Rules: fewest services → own place → longest distance at place
        let rules = Rules {
            sort: vec![Rule::SortByDifferentPlaceServices],
            filter: vec![],
        };

        let key_a = a.sort_key(date, "G", &rules);
        let key_b = b.sort_key(date, "G", &rules);

        // Alice has fewer services in a different place, usually we would prefer Bob
        // but as we need a service for Alice place, prefer Alice
        assert!(key_a < key_b, "Alice should sort before Bob");
    }

    #[test]
    fn place_counts_tracks_services_by_place() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Alice".to_string(), "A".to_string(), Rc::clone(&group_state));

        p.register_service(d(2023, 9, 1), "A".to_string());
        p.register_service(d(2023, 9, 2), "B".to_string());
        p.register_service(d(2023, 9, 3), "A".to_string());
        p.register_service(d(2023, 9, 4), "C".to_string());

        let place_counts = p.place_counts();
        assert_eq!(*place_counts.get("A").unwrap(), 2);
        assert_eq!(*place_counts.get("B").unwrap(), 1);
        assert_eq!(*place_counts.get("C").unwrap(), 1);
    }

    #[test]
    fn place_counts_decrements_on_unregister() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Bob".to_string(), "X".to_string(), Rc::clone(&group_state));

        p.register_service(d(2023, 9, 1), "X".to_string());
        p.register_service(d(2023, 9, 2), "Y".to_string());
        p.register_service(d(2023, 9, 3), "X".to_string());

        assert_eq!(*p.place_counts().get("X").unwrap(), 2);
        assert_eq!(*p.place_counts().get("Y").unwrap(), 1);

        p.unregister_service(d(2023, 9, 1), "X".to_string());

        assert_eq!(*p.place_counts().get("X").unwrap(), 1);
        assert_eq!(*p.place_counts().get("Y").unwrap(), 1);
    }

    #[test]
    fn place_counts_removes_zero_counts() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Charlie".to_string(), "P".to_string(), Rc::clone(&group_state));

        p.register_service(d(2023, 9, 1), "Q".to_string());
        assert_eq!(*p.place_counts().get("Q").unwrap(), 1);

        p.unregister_service(d(2023, 9, 1), "Q".to_string());
        assert!(!p.place_counts().contains_key("Q"));
    }

    #[test]
    fn place_counts_empty_for_new_person() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let p = PersonState::new("Dave".to_string(), "D".to_string(), Rc::clone(&group_state));

        let place_counts = p.place_counts();
        assert!(place_counts.is_empty());
    }

    #[test]
    fn place_counts_multiple_services_same_place() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Eve".to_string(), "E".to_string(), Rc::clone(&group_state));

        // Register 5 services at the same place
        for i in 1..=5 {
            p.register_service(d(2023, 9, i), "E".to_string());
        }

        let place_counts = p.place_counts();
        assert_eq!(*place_counts.get("E").unwrap(), 5);
        assert_eq!(place_counts.len(), 1);
    }

    #[test]
    fn place_counts_with_many_different_places() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Frank".to_string(), "F".to_string(), Rc::clone(&group_state));

        // Register services at 5 different places
        p.register_service(d(2023, 9, 1), "Place1".to_string());
        p.register_service(d(2023, 9, 2), "Place2".to_string());
        p.register_service(d(2023, 9, 3), "Place3".to_string());
        p.register_service(d(2023, 9, 4), "Place4".to_string());
        p.register_service(d(2023, 9, 5), "Place5".to_string());

        let place_counts = p.place_counts();
        assert_eq!(place_counts.len(), 5);
        assert_eq!(*place_counts.get("Place1").unwrap(), 1);
        assert_eq!(*place_counts.get("Place2").unwrap(), 1);
        assert_eq!(*place_counts.get("Place3").unwrap(), 1);
        assert_eq!(*place_counts.get("Place4").unwrap(), 1);
        assert_eq!(*place_counts.get("Place5").unwrap(), 1);
    }

    #[test]
    fn place_counts_unregister_nonexistent_place() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Grace".to_string(), "G".to_string(), Rc::clone(&group_state));

        p.register_service(d(2023, 9, 1), "G".to_string());

        // Try to unregister a place that was never registered
        p.unregister_service(d(2023, 9, 2), "NonExistent".to_string());

        // Original place count should be unaffected
        assert_eq!(*p.place_counts().get("G").unwrap(), 1);
        assert!(!p.place_counts().contains_key("NonExistent"));
    }

    #[test]
    fn place_counts_unregister_when_zero() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Henry".to_string(), "H".to_string(), Rc::clone(&group_state));

        // Don't register any services
        // Try to unregister - should handle gracefully
        p.unregister_service(d(2023, 9, 1), "H".to_string());

        let place_counts = p.place_counts();
        assert!(place_counts.is_empty());
    }

    #[test]
    fn place_counts_unregister_multiple_times() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Iris".to_string(), "I".to_string(), Rc::clone(&group_state));

        p.register_service(d(2023, 9, 1), "I".to_string());
        p.register_service(d(2023, 9, 2), "I".to_string());
        p.register_service(d(2023, 9, 3), "I".to_string());

        assert_eq!(*p.place_counts().get("I").unwrap(), 3);

        // Unregister twice
        p.unregister_service(d(2023, 9, 1), "I".to_string());
        assert_eq!(*p.place_counts().get("I").unwrap(), 2);

        p.unregister_service(d(2023, 9, 2), "I".to_string());
        assert_eq!(*p.place_counts().get("I").unwrap(), 1);

        // Unregister one more time - should remove the entry
        p.unregister_service(d(2023, 9, 3), "I".to_string());
        assert!(!p.place_counts().contains_key("I"));
    }

    #[test]
    fn place_counts_mixed_register_and_unregister() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Jack".to_string(), "J".to_string(), Rc::clone(&group_state));

        // Register services at two places
        p.register_service(d(2023, 9, 1), "J".to_string());
        p.register_service(d(2023, 9, 2), "K".to_string());
        p.register_service(d(2023, 9, 3), "J".to_string());
        p.register_service(d(2023, 9, 4), "K".to_string());

        assert_eq!(*p.place_counts().get("J").unwrap(), 2);
        assert_eq!(*p.place_counts().get("K").unwrap(), 2);

        // Unregister one from each
        p.unregister_service(d(2023, 9, 1), "J".to_string());
        p.unregister_service(d(2023, 9, 2), "K".to_string());

        assert_eq!(*p.place_counts().get("J").unwrap(), 1);
        assert_eq!(*p.place_counts().get("K").unwrap(), 1);

        // Register more at J
        p.register_service(d(2023, 9, 5), "J".to_string());
        assert_eq!(*p.place_counts().get("J").unwrap(), 2);
    }

    #[test]
    fn place_counts_getter_returns_clone() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Kate".to_string(), "K".to_string(), Rc::clone(&group_state));

        p.register_service(d(2023, 9, 1), "K".to_string());

        // Get a clone of place counts
        let mut counts1 = p.place_counts();
        assert_eq!(*counts1.get("K").unwrap(), 1);

        // Modify the clone
        counts1.insert("K".to_string(), 999);

        // Original should be unchanged
        let counts2 = p.place_counts();
        assert_eq!(*counts2.get("K").unwrap(), 1);
    }

    #[test]
    fn place_counts_with_special_place_names() {
        let group_state = Rc::new(RefCell::new(GroupState::default()));
        let mut p = PersonState::new("Leo".to_string(), "L".to_string(), Rc::clone(&group_state));

        // Test with special characters and spaces
        p.register_service(d(2023, 9, 1), "Place-A".to_string());
        p.register_service(d(2023, 9, 2), "Place B".to_string());
        p.register_service(d(2023, 9, 3), "Place_C".to_string());
        p.register_service(d(2023, 9, 4), "PlaceD!".to_string());

        let place_counts = p.place_counts();
        assert_eq!(*place_counts.get("Place-A").unwrap(), 1);
        assert_eq!(*place_counts.get("Place B").unwrap(), 1);
        assert_eq!(*place_counts.get("Place_C").unwrap(), 1);
        assert_eq!(*place_counts.get("PlaceD!").unwrap(), 1);
        assert_eq!(place_counts.len(), 4);
    }
}
