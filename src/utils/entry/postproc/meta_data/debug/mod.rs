use std::collections::HashMap;
use crate::utils::response::{ResponseResult};
use crate::utils::entry::{Maybe, Entry, EntryValue};

pub fn debug(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<Entry> {
    let mut view: Vec<Entry> = Vec::new();

    for (key,value) in maybes {
        match value {
            Maybe { data: Ok(resolved), timestamp } => {
                view.push(Entry {
                    timestamp: timestamp.to_owned(),
                    origin: key.to_owned(),
                    value: EntryValue::Value(serde_json::json!({
                        "data": format!("{:?}",resolved),
                        "group":Some("[DEBUG]".to_string())
                    }))
                });
            }
            Maybe { data: Err(_), .. } => {}
        }
    }
    view
}
