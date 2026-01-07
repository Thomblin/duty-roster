//! executable part of this library. read configuration and generate a csv file with a schedule

use clap::Parser;
use duty_roster::{
    PersonState,
    config::load_config,
    csv::assignments_to_csv,
    dates::get_weekdays,
    gui,
    schedule::{Assignment, create_schedule},
};
use std::{error::Error, fs::File, io::Write};

/// Duty Roster - Generate and manage schedules for people and places/tasks
///
/// By default, this application runs in GUI mode. Use the --cli flag to run in command-line mode.
#[derive(Parser, Debug)]
#[command(version, about = "Duty Roster - Generate and manage schedules for people and places/tasks", long_about = None)]
struct Args {
    /// file with settings to create the schedule
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// filename of csv to generate
    #[arg(short, long, default_value = "schedule.csv")]
    out: String,

    /// run in CLI mode (no GUI)
    #[arg(short = 'C', long)]
    cli: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.cli {
        // Run in CLI mode
        println!("Running in CLI mode...");
        let config = load_config(&args.config)?;
        let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);

        let (assignments, people) = create_schedule(&dates, &config);

        match store_csv(assignments, people, &args.out) {
            Ok(_) => println!("stored schedule to {}", args.out),
            Err(e) => println!("error: could not store results: {e:?}"),
        };
    } else {
        // Run in GUI mode (default)
        println!("Starting GUI mode...");
        gui::run()?;
    }

    Ok(())
}

fn store_csv(
    assignments: Vec<Assignment>,
    people: Vec<PersonState>,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(filename)?;

    file.write_all(assignments_to_csv(&assignments)?.as_bytes())
        .unwrap_or_else(|e| panic!("could not write to file {filename}: {e:?}"));

    file.write_all(b"\n")?;

    for person in people {
        file.write_all(
            format!("{}, total: {}", person.name(), person.total_services()).as_bytes(),
        )?;

        for (day, count) in person.weekday_counts() {
            file.write_all(format!(", {day}: {count}").as_bytes())?;
        }
        file.write_all(
            format!(", different_place: {}\n", person.different_place_services()).as_bytes(),
        )?;
    }

    Ok(())
}
