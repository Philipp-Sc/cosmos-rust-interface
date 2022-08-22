
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::hash::{Hash, Hasher};

#[cfg(feature = "postproc")]
pub mod postproc;

#[cfg(feature = "db")]
pub mod db;


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
    Value(serde_json::Value)
}

// type used by the post processing to describe a data point that can be passed on to the visualisation component
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Entry {
    pub timestamp: i64,
    pub origin: String,
    pub value: EntryValue
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
        self.origin.hash(state);
        format!("{:?}",self.value).hash(state);
    }
}

impl Entry {
    pub fn new(timestamp: i64, key: &str, value: EntryValue) -> Entry {
        Entry {
            timestamp,
            origin: key.to_string(),
            value,
        }
    }
}