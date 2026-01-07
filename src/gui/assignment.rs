use chrono::NaiveDate;

use crate::schedule::{Assignment, PersonState};

/// Swap assignments between two positions
#[allow(clippy::too_many_arguments)]
pub fn swap_assignments(
    assignments: &mut [Assignment],
    people: &mut [PersonState],
    date1: NaiveDate,
    place1: &str,
    person1: &str,
    date2: NaiveDate,
    place2: &str,
    person2: &str,
) -> bool {
    // First, find if both assignments exist
    let mut found1 = false;
    let mut found2 = false;

    // Check if both assignments exist
    for assignment in assignments.iter() {
        if assignment.date == date1 && assignment.place == place1 {
            found1 = true;
        } else if assignment.date == date2 && assignment.place == place2 {
            found2 = true;
        }

        if found1 && found2 {
            break;
        }
    }
    
    // Only update if both assignments are found
    if found1 && found2 {
        for assignment in assignments {
            if assignment.date == date1 && assignment.place == place1 {
                assignment.person = person2.to_string();
            } else if assignment.date == date2 && assignment.place == place2 {
                assignment.person = person1.to_string();
            }
        }
    }

    // Update the people's service records
    if found1 && found2 {
        // Find the indices of the people involved
        let person1_idx = people.iter().position(|p| p.name() == person1);
        let person2_idx = people.iter().position(|p| p.name() == person2);

        // Update the service records if both people are found
        if let (Some(idx1), Some(idx2)) = (person1_idx, person2_idx) {
            // We need to be careful with borrowing rules here
            // First, unregister the old services
            {
                let p1 = &mut people[idx1];
                p1.unregister_service(date1, place1.to_string());
            }
            {
                let p2 = &mut people[idx2];
                p2.unregister_service(date2, place2.to_string());
            }

            // Then register the new services
            {
                let p1 = &mut people[idx1];
                p1.register_service(date2, place2.to_string());
            }
            {
                let p2 = &mut people[idx2];
                p2.register_service(date1, place1.to_string());
            }
        }
    }

    found1 && found2
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::cell::RefCell;
    use std::rc::Rc;
    use crate::schedule::GroupState;

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

    fn create_test_people() -> Vec<PersonState> {
        let group_state1 = Rc::new(RefCell::new(GroupState::default()));
        let group_state2 = Rc::new(RefCell::new(GroupState::default()));
        
        let mut person1 = PersonState::new(
            "Person1".to_string(),
            "Place A".to_string(),
            Rc::clone(&group_state1),
        );
        
        let mut person2 = PersonState::new(
            "Person2".to_string(),
            "Place B".to_string(),
            Rc::clone(&group_state2),
        );
        
        // Register initial services
        person1.register_service(create_test_date(2025, 9, 1), "Place A".to_string());
        person2.register_service(create_test_date(2025, 9, 2), "Place B".to_string());
        
        vec![person1, person2]
    }

    #[test]
    fn test_swap_assignments_success() {
        let mut assignments = create_test_assignments();
        let mut people = create_test_people();
        
        // Before swap
        assert_eq!(assignments[0].person, "Person1");
        assert_eq!(assignments[1].person, "Person2");
        assert_eq!(people[0].total_services(), 1);
        assert_eq!(people[1].total_services(), 1);
        
        // Perform swap
        let result = swap_assignments(
            &mut assignments,
            &mut people,
            create_test_date(2025, 9, 1), "Place A", "Person1",
            create_test_date(2025, 9, 2), "Place B", "Person2"
        );
        
        // Check result
        assert!(result);
        
        // Check assignments were updated
        assert_eq!(assignments[0].person, "Person2");
        assert_eq!(assignments[1].person, "Person1");
        
        // Check people stats were updated
        assert_eq!(people[0].total_services(), 1); // Should remain 1 (unregistered 1, registered 1)
        assert_eq!(people[1].total_services(), 1); // Should remain 1 (unregistered 1, registered 1)
    }

    #[test]
    fn test_swap_assignments_not_found() {
        let mut assignments = create_test_assignments();
        let mut people = create_test_people();
        
        // Try to swap with non-existent assignment
        let result = swap_assignments(
            &mut assignments,
            &mut people,
            create_test_date(2025, 9, 1), "Place A", "Person1",
            create_test_date(2025, 9, 3), "Place C", "Person3" // Non-existent
        );
        
        // Check result
        assert!(!result);
        
        // Check assignments were not changed
        assert_eq!(assignments[0].person, "Person1"); // First assignment still has Person1
        assert_eq!(assignments[1].person, "Person2"); // Second assignment still has Person2
    }
}
