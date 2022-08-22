// save / load / format & display Entries
// only cosmos-rust-interface should know about Entries! and possibly cosmos-rust-bot
pub mod query;

use std::fs;
use crate::utils::entry::Entry;

pub fn save_entries(path: &str, entries: Vec<Entry>) {
    let line = format!("{}", serde_json::to_string(&entries).unwrap());
    fs::write(path, &line).ok();
}

pub async fn load_entries(path: &str) -> Option<Vec<Entry>> {
    let mut entries: Option<Vec<Entry>> = None;
    let mut try_counter = 0;
    while entries.is_none() && try_counter < 3 {
        match fs::read_to_string(path) {
            Ok(file) => {
                match serde_json::from_str(&file) {
                    Ok(res) => { entries = Some(res); }
                    Err(_) => { try_counter = try_counter + 1; }
                };
            }
            Err(_) => {
                try_counter = try_counter + 1;
            }
        }
    }
    entries
}