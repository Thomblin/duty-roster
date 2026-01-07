//! Library and Binary to create a schedule for a given set of people for a given set of tasks/places during a given range of dates
//! for an example how to use: see main.rs

pub mod config;
pub mod csv;
pub mod dates;
pub mod gui;
pub mod schedule;

pub use schedule::PersonState;
