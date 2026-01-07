use chrono::NaiveDate;

use crate::schedule::{Assignment, PersonState};

/// Swap assignments between two positions
pub fn swap_assignments(
    assignments: &mut [Assignment], 
    _people: &mut [PersonState],
    date1: NaiveDate, place1: &str, person1: &str,
    date2: NaiveDate, place2: &str, person2: &str
) -> bool {
    // Find and update the assignments
    let mut found1 = false;
    let mut found2 = false;
    
    // Update assignments
    for assignment in assignments {
        if assignment.date == date1 && assignment.place == place1 {
            assignment.person = person2.to_string();
            found1 = true;
        } else if assignment.date == date2 && assignment.place == place2 {
            assignment.person = person1.to_string();
            found2 = true;
        }
        
        if found1 && found2 {
            break;
        }
    }
    
    // Update the people's service records
    if found1 && found2 {
        // In a real implementation, we would need to add unregister_service method to PersonState
        // and update the service records here. For now, we'll just return success.
        
        // This is a conceptual implementation that would work if we had unregister_service:
        // 
        // We can't do this with two separate iter_mut() calls due to borrowing rules
        // Instead, we would need to find the indices first, then update them
        // 
        // let person1_idx = people.iter().position(|p| p.name() == person1);
        // let person2_idx = people.iter().position(|p| p.name() == person2);
        // 
        // if let (Some(idx1), Some(idx2)) = (person1_idx, person2_idx) {
        //     let p1 = &mut people[idx1];
        //     let p2 = &mut people[idx2];
        //     p1.unregister_service(date1, place1.to_string());
        //     p2.unregister_service(date2, place2.to_string());
        //     p1.register_service(date2, place2.to_string());
        //     p2.register_service(date1, place1.to_string());
        // }
    }
    
    found1 && found2
}
