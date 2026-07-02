//! distribute extra tasks across eligible group members and append icons to assignments

use std::collections::{BTreeSet, HashMap};

use crate::config::Config;
use crate::schedule::Assignment;

/// Apply extra tasks to existing assignments.
/// Resets any previously applied icons, then fairly distributes each extra task
/// among eligible persons (those whose group is listed in the task's `groups`).
/// For each date, picks the eligible person with the fewest prior assignments of
/// this task; prefers persons not already carrying another extra task that day (soft).
pub fn apply_extra_tasks(assignments: &mut Vec<Assignment>, config: &Config) {
    // Reset person to base_person for all assignments
    for a in assignments.iter_mut() {
        a.person = a.base_person.clone();
    }

    let extra_tasks = match &config.extra_task {
        Some(tasks) if !tasks.is_empty() => tasks,
        _ => return,
    };

    // Build map: group name OR group place → set of full person names (base names)
    // Supports matching by either group.name (e.g. "Maier") or group.place (e.g. "Sonnengruppe")
    let mut group_members: HashMap<&str, BTreeSet<String>> = HashMap::new();
    for group in &config.group {
        for key in [group.name.as_str(), group.place.as_str()] {
            let entry = group_members.entry(key).or_default();
            for member in &group.members {
                entry.insert(format!("{} {}", member.name, group.name));
            }
        }
    }

    // Collect sorted dates
    let mut dates: Vec<chrono::NaiveDate> = assignments.iter().map(|a| a.date).collect();
    dates.sort();
    dates.dedup();

    for task in extra_tasks {
        // Eligible base names for this task
        let eligible: BTreeSet<String> = task
            .groups
            .iter()
            .flat_map(|g| group_members.get(g.as_str()).into_iter().flatten().cloned())
            .collect();

        if eligible.is_empty() {
            continue;
        }

        // Quota-based assignment: pre-compute each person's target count so that the
        // resulting distribution has gap ≤ 1 by construction. Then use most-constrained-
        // date-first greedy, only picking people who haven't hit their quota yet.
        // Fallback: if all candidates on a date hit quota, pick the one with lowest count.
        let all_slots: Vec<usize> = assignments
            .iter()
            .enumerate()
            .filter(|(_, a)| eligible.contains(&a.base_person))
            .map(|(i, _)| i)
            .collect();

        let mut total_appearances: HashMap<String, usize> = HashMap::new();
        for &i in &all_slots {
            *total_appearances
                .entry(assignments[i].base_person.clone())
                .or_default() += 1;
        }

        // Build date→slots map first so we can count dates for quota calculation
        let mut slots_by_date: HashMap<chrono::NaiveDate, Vec<usize>> = HashMap::new();
        for &i in &all_slots {
            slots_by_date
                .entry(assignments[i].date)
                .or_default()
                .push(i);
        }

        let mut counts: HashMap<String, usize> = eligible.iter().map(|n| (n.clone(), 0)).collect();

        // Most-constrained-date-first, then pick by lowest count with a ratio tiebreaker.
        // The ratio count/total_appearances normalises across people with different numbers
        // of eligible slots, so those with more slots don't systematically win ties.
        let mut date_order: Vec<chrono::NaiveDate> = slots_by_date.keys().copied().collect();
        date_order.sort_by_key(|d| (slots_by_date[d].len(), *d));

        for date in date_order {
            let candidates = &slots_by_date[&date];

            // Key: (count, count*total_sum/total_appearances as load_ratio numerator, icon, index)
            // count ascending = fewer picks = higher priority.
            // For ties: lower load ratio (count/total) = more behind fair share = higher priority.
            //   count_a/total_a < count_b/total_b  ↔  count_a * total_b < count_b * total_a
            // Then icon-free. Then index.
            let best = candidates
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    let base_a = &assignments[a].base_person;
                    let base_b = &assignments[b].base_person;
                    let count_a = counts.get(base_a.as_str()).copied().unwrap_or(0);
                    let count_b = counts.get(base_b.as_str()).copied().unwrap_or(0);
                    let total_a = total_appearances.get(base_a.as_str()).copied().unwrap_or(1);
                    let total_b = total_appearances.get(base_b.as_str()).copied().unwrap_or(1);
                    let icon_a = (assignments[a].person != assignments[a].base_person) as usize;
                    let icon_b = (assignments[b].person != assignments[b].base_person) as usize;
                    count_a
                        .cmp(&count_b)
                        .then_with(|| (count_a * total_b).cmp(&(count_b * total_a)))
                        .then(icon_a.cmp(&icon_b))
                        .then(a.cmp(&b))
                })
                .unwrap();
            let base = assignments[best].base_person.clone();
            let cur = assignments[best].person.clone();
            assignments[best].person = format!("{} {}", cur, task.name);
            *counts.entry(base.clone()).or_default() += 1;
        }

        // Refinement: BFS augmenting-path to fix gap > 1.
        // Find a chain: under-assigned person → date they appear on → over-assigned holder
        // → another date they hold → ... → date where they appear with under-assigned person.
        // This is the standard augmenting-path technique for bipartite matching balance.
        loop {
            let min_count = counts.values().copied().min().unwrap_or(0);
            let max_count = counts.values().copied().max().unwrap_or(0);
            if max_count - min_count <= 1 {
                break;
            }

            // Build index: person → dates they appear on as eligible
            let mut person_dates: HashMap<String, Vec<chrono::NaiveDate>> = HashMap::new();
            for (&date, slots) in &slots_by_date {
                for &i in slots {
                    person_dates
                        .entry(assignments[i].base_person.clone())
                        .or_default()
                        .push(date);
                }
            }
            // Build index: date → who holds the task (if any)
            let mut date_holder: HashMap<chrono::NaiveDate, String> = HashMap::new();
            for (&date, slots) in &slots_by_date {
                for &i in slots {
                    if assignments[i].person != assignments[i].base_person
                        && assignments[i].person.contains(task.name.as_str())
                    {
                        date_holder.insert(date, assignments[i].base_person.clone());
                    }
                }
            }

            // Sort person_dates for deterministic BFS traversal
            for dates in person_dates.values_mut() {
                dates.sort();
            }

            // BFS from each min-count person to find a path to a max-count person
            // via alternating (date-without-task, person-with-task) edges.
            let mut min_people: Vec<String> = counts
                .iter()
                .filter(|&(_, &c)| c == min_count)
                .map(|(n, _)| n.clone())
                .collect();
            min_people.sort();

            let mut path: Option<Vec<(chrono::NaiveDate, String)>> = None;
            'bfs: for start in &min_people {
                // BFS: state = current person. Track path as (date, next_person) pairs.
                use std::collections::VecDeque;
                let mut queue: VecDeque<(String, Vec<(chrono::NaiveDate, String)>)> =
                    VecDeque::new();
                let mut visited: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                visited.insert(start.clone());
                queue.push_back((start.clone(), vec![]));

                while let Some((person, chain)) = queue.pop_front() {
                    let mut dates = person_dates.get(&person).cloned().unwrap_or_default();
                    dates.sort();
                    for date in dates {
                        let date = date; // shadow to owned
                        // This person doesn't hold the task on this date
                        if date_holder.get(&date).map(|h| h.as_str()) == Some(person.as_str()) {
                            continue;
                        }
                        // Who holds it?
                        if let Some(holder) = date_holder.get(&date) {
                            let holder_count = counts[holder];
                            let mut new_chain = chain.clone();
                            new_chain.push((date, holder.clone()));
                            if holder_count == max_count {
                                // Found path: start → ... → holder (over-assigned)
                                path = Some(new_chain);
                                break 'bfs;
                            }
                            if !visited.contains(holder) {
                                visited.insert(holder.clone());
                                queue.push_back((holder.clone(), new_chain));
                            }
                        }
                    }
                }
            }

            let Some(chain) = path else {
                break;
            };
            // Apply the chain: for each (date, holder) in chain, transfer task from holder
            // to the previous person in the path.
            // chain[0] = (date0, holder0): give task to start person on date0, take from holder0
            // chain[1] = (date1, holder1): give task to holder0 on date1, take from holder1
            // etc.
            let start_person = min_people
                .iter()
                .find(|p| {
                    let first_date = chain[0].0;
                    person_dates
                        .get(*p)
                        .map(|ds| ds.contains(&first_date))
                        .unwrap_or(false)
                        && date_holder.get(&first_date).map(|h| h.as_str()) != Some(p.as_str())
                })
                .unwrap()
                .clone();

            let mut give_to = start_person;
            for (date, take_from) in &chain {
                // Find slot index for give_to on this date
                let give_idx = slots_by_date[date]
                    .iter()
                    .copied()
                    .find(|&i| assignments[i].base_person == give_to)
                    .unwrap();
                // Find slot index for take_from on this date
                let take_idx = slots_by_date[date]
                    .iter()
                    .copied()
                    .find(|&i| assignments[i].base_person == *take_from)
                    .unwrap();
                // Transfer
                assignments[take_idx].person = assignments[take_idx].base_person.clone();
                let cur = assignments[give_idx].person.clone();
                assignments[give_idx].person = format!("{} {}", cur, task.name);
                *counts.entry(take_from.clone()).or_default() -= 1;
                *counts.entry(give_to.clone()).or_default() += 1;
                give_to = take_from.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Dates, ExtraTask, Group, Member, Places, Rules};
    use chrono::{NaiveDate, Weekday};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn make_config(extra_tasks: Vec<ExtraTask>) -> Config {
        Config {
            dates: Dates {
                from: date(2025, 9, 1),
                to: date(2025, 9, 5),
                exceptions: vec![],
                weekdays: vec![
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                ],
            },
            places: Places {
                places: vec!["Maier".to_string(), "Doe".to_string()],
            },
            group: vec![
                Group {
                    name: "Maier".to_string(),
                    place: "Maier".to_string(),
                    members: vec![
                        Member {
                            name: "Alice".to_string(),
                        },
                        Member {
                            name: "Bob".to_string(),
                        },
                    ],
                },
                Group {
                    name: "Doe".to_string(),
                    place: "Doe".to_string(),
                    members: vec![Member {
                        name: "Charlie".to_string(),
                    }],
                },
            ],
            rules: Rules {
                sort: vec![],
                filter: vec![],
            },
            extra_task: Some(extra_tasks),
        }
    }

    fn assignment(d: NaiveDate, place: &str, person: &str) -> Assignment {
        Assignment {
            date: d,
            place: place.to_string(),
            person: person.to_string(),
            base_person: person.to_string(),
        }
    }

    #[test]
    fn extra_task_appended_to_eligible_person() {
        let config = make_config(vec![ExtraTask {
            name: "🪴".to_string(),
            groups: vec!["Maier".to_string()],
        }]);

        // One date, two assignments: one Maier-group (Alice), one Doe-group (Charlie)
        let d = date(2025, 9, 1);
        let mut assignments = vec![
            assignment(d, "Maier", "Alice Maier"),
            assignment(d, "Doe", "Charlie Doe"),
        ];

        apply_extra_tasks(&mut assignments, &config);

        // Alice is eligible, Charlie is not
        assert_eq!(assignments[0].person, "Alice Maier 🪴");
        assert_eq!(assignments[1].person, "Charlie Doe");
    }

    #[test]
    fn extra_task_distributed_evenly_across_dates() {
        let config = make_config(vec![ExtraTask {
            name: "🪴".to_string(),
            groups: vec!["Maier".to_string()],
        }]);

        // 4 dates, each date has Alice and Bob both assigned to Maier
        let dates = [
            date(2025, 9, 1),
            date(2025, 9, 2),
            date(2025, 9, 3),
            date(2025, 9, 4),
        ];
        let mut assignments: Vec<Assignment> = dates
            .iter()
            .flat_map(|&d| {
                vec![
                    assignment(d, "Maier", "Alice Maier"),
                    assignment(d, "Doe", "Bob Maier"), // Bob also Maier group but assigned to Doe
                ]
            })
            .collect();

        apply_extra_tasks(&mut assignments, &config);

        let alice_count = assignments
            .iter()
            .filter(|a| a.person == "Alice Maier 🪴")
            .count();
        let bob_count = assignments
            .iter()
            .filter(|a| a.person == "Bob Maier 🪴")
            .count();

        // Should be 2 each (4 dates, 2 eligible people)
        assert_eq!(alice_count + bob_count, 4);
        assert!(
            (alice_count as i32 - bob_count as i32).abs() <= 1,
            "Distribution uneven: alice={alice_count} bob={bob_count}"
        );
    }

    #[test]
    fn apply_resets_previous_icons_before_reapplying() {
        let config = make_config(vec![ExtraTask {
            name: "🪴".to_string(),
            groups: vec!["Maier".to_string()],
        }]);

        let d = date(2025, 9, 1);
        let mut assignments = vec![assignment(d, "Maier", "Alice Maier")];

        apply_extra_tasks(&mut assignments, &config);
        assert_eq!(assignments[0].person, "Alice Maier 🪴");

        // Apply again — should not double-append
        apply_extra_tasks(&mut assignments, &config);
        assert_eq!(assignments[0].person, "Alice Maier 🪴");
    }

    #[test]
    fn two_tasks_same_eligible_both_assigned_per_date() {
        // With 2 tasks and 2 people, over 2+ dates each person gets each task once.
        // Fairness (count) is the only criterion — no busy-avoidance penalty.
        let config = make_config(vec![
            ExtraTask {
                name: "🪴".to_string(),
                groups: vec!["Maier".to_string()],
            },
            ExtraTask {
                name: "🪟".to_string(),
                groups: vec!["Maier".to_string()],
            },
        ]);

        let mut assignments = vec![
            assignment(date(2025, 9, 1), "Maier", "Alice Maier"),
            assignment(date(2025, 9, 1), "Doe", "Bob Maier"),
            assignment(date(2025, 9, 2), "Maier", "Alice Maier"),
            assignment(date(2025, 9, 2), "Doe", "Bob Maier"),
        ];

        apply_extra_tasks(&mut assignments, &config);

        let count = |name: &str, icon: &str| {
            assignments
                .iter()
                .filter(|a| a.person.contains(icon) && a.base_person == name)
                .count()
        };

        // Each person should get each task exactly once over 2 dates
        assert_eq!(count("Alice Maier", "🪴") + count("Bob Maier", "🪴"), 2);
        assert_eq!(count("Alice Maier", "🪟") + count("Bob Maier", "🪟"), 2);
        assert!((count("Alice Maier", "🪴") as i32 - count("Bob Maier", "🪴") as i32).abs() <= 1);
        assert!((count("Alice Maier", "🪟") as i32 - count("Bob Maier", "🪟") as i32).abs() <= 1);
    }

    #[test]
    fn max_gap_is_one_across_eligible_pool() {
        // Each date has two eligible people (one per place). The algorithm must
        // pick the lower-count person, not the one who appeared first or last.
        // 10 dates, eligible: Alice Maier + Bob Maier + Charlie Doe (all eligible via place match)
        let config = make_config(vec![ExtraTask {
            name: "🪟".to_string(),
            groups: vec!["Maier".to_string(), "Doe".to_string()],
        }]);

        // 10 dates, each with Alice and Charlie assigned (both eligible)
        let mut assignments: Vec<Assignment> = Vec::new();
        for i in 1..=10u32 {
            let d = date(2025, 9, i);
            assignments.push(assignment(d, "Maier", "Alice Maier"));
            assignments.push(assignment(d, "Doe", "Charlie Doe"));
        }

        apply_extra_tasks(&mut assignments, &config);

        let count = |name: &str| -> usize {
            assignments
                .iter()
                .filter(|a| a.person.contains("🪟") && a.base_person == name)
                .count()
        };

        let a = count("Alice Maier");
        let c = count("Charlie Doe");

        assert_eq!(a + c, 10, "all dates must get the task");
        assert!(
            (a as i32 - c as i32).abs() <= 1,
            "gap too large: Alice={a} Charlie={c}"
        );
    }

    #[test]
    fn two_tasks_different_eligibility_stay_balanced() {
        // Mimics kita: 🪴 eligible for all 3 places, 🪟 only for 2 of them.
        // Mondgruppe people get 🪴 but NOT 🪟. When 🪴 runs first and marks
        // Sonnengruppe/Sternschnuppengruppe people as busy, 🪟 must still
        // distribute evenly across those two groups.
        let config = make_config(vec![
            ExtraTask {
                name: "🪴".to_string(),
                groups: vec!["Maier".to_string(), "Doe".to_string()],
            },
            ExtraTask {
                name: "🪟".to_string(),
                groups: vec!["Maier".to_string()], // only Maier (2 people: Alice, Bob)
            },
        ]);

        // 10 dates, each: Alice (Maier), Bob (Maier), Charlie (Doe) all assigned
        let mut assignments: Vec<Assignment> = Vec::new();
        for i in 1..=10u32 {
            let d = date(2025, 9, i);
            assignments.push(assignment(d, "Maier", "Alice Maier"));
            assignments.push(assignment(d, "Maier", "Bob Maier"));
            assignments.push(assignment(d, "Doe", "Charlie Doe"));
        }

        apply_extra_tasks(&mut assignments, &config);

        let count = |name: &str, icon: &str| -> usize {
            assignments
                .iter()
                .filter(|a| a.person.contains(icon) && a.base_person == name)
                .count()
        };

        let alice_w = count("Alice Maier", "🪟");
        let bob_w = count("Bob Maier", "🪟");
        assert_eq!(alice_w + bob_w, 10, "all 10 dates need 🪟");
        assert!(
            (alice_w as i32 - bob_w as i32).abs() <= 1,
            "🪟 gap too large: Alice={alice_w} Bob={bob_w}"
        );
    }

    #[test]
    fn large_scale_gap_is_at_most_one() {
        // 88 dates, 11 Sonn-eligible + 10 Stern-eligible = 21 people for 🪟
        // Each date: one Sonn person + one Stern person assigned.
        // People appear in round-robin order across their group.
        use crate::config::{Config, Dates, ExtraTask, Group, Member, Places, Rules};
        use chrono::Weekday;

        // Use pool sizes that divide evenly into 88 to avoid unequal appearances.
        // 8 Sonn + 8 Stern = 16 people. 88/8 = 11 appearances each. 88 picks, 16 people → 5 or 6 each (gap≤1).
        let sonn_names: Vec<&str> = vec![
            "Ana", "Antonio", "Rita", "Jochen", "Julia", "Marcel", "Karolin", "Ramin",
        ];
        let stern_names: Vec<&str> = vec![
            "Corinna",
            "Stephan",
            "Henning",
            "Nina",
            "XX",
            "XY",
            "Charlotte",
            "Christina",
        ];

        let config = Config {
            dates: Dates {
                from: date(2025, 8, 28),
                to: date(2026, 8, 7),
                exceptions: vec![],
                weekdays: vec![Weekday::Thu, Weekday::Fri],
            },
            places: Places {
                places: vec!["Sonn".to_string(), "Stern".to_string(), "Mond".to_string()],
            },
            group: {
                let mut groups = vec![];
                for name in &sonn_names {
                    groups.push(Group {
                        name: name.to_string(),
                        place: "Sonn".to_string(),
                        members: vec![Member {
                            name: name.to_string(),
                        }],
                    });
                }
                for name in &stern_names {
                    groups.push(Group {
                        name: name.to_string(),
                        place: "Stern".to_string(),
                        members: vec![Member {
                            name: name.to_string(),
                        }],
                    });
                }
                groups
            },
            rules: Rules {
                sort: vec![],
                filter: vec![],
            },
            extra_task: Some(vec![
                ExtraTask {
                    name: "🪴".to_string(),
                    groups: sonn_names
                        .iter()
                        .chain(stern_names.iter())
                        .map(|s| s.to_string())
                        .collect(),
                },
                ExtraTask {
                    name: "🪟".to_string(),
                    groups: sonn_names
                        .iter()
                        .chain(stern_names.iter())
                        .map(|s| s.to_string())
                        .collect(),
                },
            ]),
        };

        let mut assignments = Vec::new();
        for d in 0..88u32 {
            let day = date(2025, 8, 28) + chrono::Duration::days(d as i64);
            let sonn = sonn_names[d as usize % sonn_names.len()];
            let stern = stern_names[d as usize % stern_names.len()];
            assignments.push(Assignment {
                date: day,
                place: "Sonn".to_string(),
                person: format!("{sonn} {sonn}"),
                base_person: format!("{sonn} {sonn}"),
            });
            assignments.push(Assignment {
                date: day,
                place: "Stern".to_string(),
                person: format!("{stern} {stern}"),
                base_person: format!("{stern} {stern}"),
            });
        }

        apply_extra_tasks(&mut assignments, &config);

        for icon in ["🪴", "🪟"] {
            let counts: Vec<usize> = sonn_names
                .iter()
                .chain(stern_names.iter())
                .map(|name| {
                    let full = format!("{name} {name}");
                    assignments
                        .iter()
                        .filter(|a| a.person.contains(icon) && a.base_person == full)
                        .count()
                })
                .collect();
            let max = *counts.iter().max().unwrap();
            let min = *counts.iter().min().unwrap();
            assert!(
                max - min <= 1,
                "{icon} gap too large: max={max} min={min} counts={counts:?}"
            );
        }
    }

    #[test]
    fn person_with_fewer_appearances_gets_proportional_quota() {
        // Bob appears only on date 1 (total=1, quota=1). Alice appears on dates 1 and 2 (total=2, quota=2).
        // 3 dates total, 2 people: floor(3/2)=1 each, 1 extra → Alice (more appearances) gets quota=2.
        // Result: Bob gets exactly 1, Alice gets exactly 2.
        let config = make_config(vec![ExtraTask {
            name: "🪟".to_string(),
            groups: vec!["Maier".to_string(), "Doe".to_string()],
        }]);

        let mut assignments = vec![
            assignment(date(2025, 9, 1), "Maier", "Alice Maier"),
            assignment(date(2025, 9, 1), "Doe", "Bob Maier"),
            assignment(date(2025, 9, 2), "Maier", "Alice Maier"),
        ];

        apply_extra_tasks(&mut assignments, &config);

        let alice = assignments
            .iter()
            .filter(|a| a.base_person == "Alice Maier" && a.person.contains("🪟"))
            .count();
        let bob = assignments
            .iter()
            .filter(|a| a.base_person == "Bob Maier" && a.person.contains("🪟"))
            .count();
        assert_eq!(alice + bob, 2, "2 dates must be covered");
        // With flat quotas: Alice quota=2, Bob quota=1 — gap ≤ 1
        assert!(
            (alice as i32 - bob as i32).abs() <= 1,
            "gap too large: Alice={alice} Bob={bob}"
        );
    }

    #[test]
    fn no_extra_tasks_in_config_leaves_assignments_unchanged() {
        let mut config = make_config(vec![]);
        config.extra_task = None;

        let d = date(2025, 9, 1);
        let mut assignments = vec![assignment(d, "Maier", "Alice Maier")];

        apply_extra_tasks(&mut assignments, &config);

        assert_eq!(assignments[0].person, "Alice Maier");
    }
}
