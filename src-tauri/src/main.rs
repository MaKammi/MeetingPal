#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if let Err(error) = meetingpal_lib::entry() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
