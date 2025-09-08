use clap::Parser;
use duty_roster::{
    PersonState,
    config::load_config,
    csv::assignments_to_csv,
    dates::get_weekdays,
    schedule::{Assignment, create_schedule},
};
use std::{error::Error, fs::File, io::Write};

/// Generate a schedule for given people and places/tasks
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file with settings to create the schedule
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// filename of csv to generate
    #[arg(short, long, default_value = "schedule.csv")]
    out: String,
}

fn main() {
    let args = Args::parse();

    let config = load_config(&args.config).unwrap();
    let dates = get_weekdays(&config.dates.from, &config.dates.to, &config.dates.weekdays);

    let (assignments, people) = create_schedule(&dates, &config);

    match store_csv(assignments, people, &args.out) {
        Ok(_) => println!("stored schedule to {}", args.out),
        Err(e) => println!("error: could not store results: {e:?}"),
    };
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
