/*
 * Helper functions to generate Entries through the combination/processing of ResponseResults.
 * Entries are ready to be formatted and then displayed to the user.
 *
 * In: HashMap<String,  Maybe<ResponseResult>>>, serde_json::Value
 * Out: Vec<Entry>
 */
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::hash::{Hash, Hasher};


pub mod meta_data;
pub mod blockchain;

// type used by cosmos-rust-bot describe the state of a task
#[derive(Debug)]
pub struct Maybe<T> {
    pub data: anyhow::Result<T>,
    pub timestamp: i64,
}

impl<T: Clone> Clone for Maybe<T> {
    fn clone(&self) -> Maybe<T> {
        match self {
            Maybe { data: Err(err), timestamp } => Maybe { data: Err(anyhow::anyhow!(err.to_string())), timestamp: *timestamp },
            Maybe { data: Ok(value), timestamp } => Maybe { data: Ok(value.clone()), timestamp: *timestamp },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum EntryValue {
    Text(String),
    Json(String),
}

// type used by the post processing to describe a data point that can be passed on to the visualisation component
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Entry {
    pub timestamp: i64,
    pub key: String,
    pub value: EntryValue
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
        self.key.hash(state);
        format!("{:?}",self.value).hash(state);
    }
}

impl Entry {
    pub fn new(timestamp: i64, key: &str, value: EntryValue) -> Entry {
        Entry {
            timestamp,
            key: key.to_string(),
            value,
        }
    }
}

// helper function to load a Entries from disk
pub async fn load_state(path: &str) -> Option<Vec<Option<Entry>>> {
    let mut state: Option<Vec<Option<Entry>>> = None;
    let mut try_counter = 0;
    while state.is_none() && try_counter < 3 {
        match fs::read_to_string(path) {
            Ok(file) => {
                match serde_json::from_str(&file) {
                    Ok(res) => { state = Some(res); }
                    Err(_) => { try_counter = try_counter + 1; }
                };
            }
            Err(_) => {
                try_counter = try_counter + 1;
            }
        }
    }
    if let Some(mut s) = state {
        //s.sort_by(|a, b| a.as_ref().unwrap().index.unwrap_or(0i32).cmp(&b.as_ref().unwrap().index.unwrap_or(0i32)));
        return Some(s);
    }
    state
}