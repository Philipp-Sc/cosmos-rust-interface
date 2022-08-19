use std::collections::HashMap;
use crate::utils::response::{ResponseResult};
use crate::utils::entry::{Maybe, Entry, EntryValue};


pub fn errors(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<Entry> {
    let mut view: Vec<Entry> = Vec::new();

    for (key,value) in maybes {
        match value {
            Maybe { data: Ok(_resolved), .. } => {}
            Maybe { data: Err(err), timestamp } => {
                let mut group: Option<String> = None;
                if err.to_string() == "Error: Not yet resolved!".to_string() {
                    group = Some("[Unresolved]".to_string());
                } else if err.to_string() != "Error: Entry reserved!" {
                    group = Some("[Errors]".to_string());
                }
                view.push(Entry {
                    timestamp: timestamp.to_owned(),
                    key: key.to_owned(),
                    value: EntryValue::Value(serde_json::json!({
                        "data": err.to_string(),
                        "group": group
                    }))
                });
            }
        }
    }
    view
}