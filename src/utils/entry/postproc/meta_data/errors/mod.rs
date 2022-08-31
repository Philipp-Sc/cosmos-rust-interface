use std::collections::HashMap;
use crate::utils::response::{ResponseResult};
use crate::utils::entry::{Maybe, Entry, EntryValue};


pub fn errors(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<Entry> {
    let mut view: Vec<Entry> = Vec::new();

    for (key,value) in maybes {
        match value {
            Maybe { data: Ok(_resolved), .. } => {}
            Maybe { data: Err(err), timestamp } => {
                let mut group: String = "unknown".to_string();
                if err.to_string() == "Error: Not yet resolved!".to_string() {
                    group = "unresolved".to_string();
                } else if err.to_string() != "Error: Entry reserved!" {
                    group = "errors".to_string();
                }
                let filter = serde_json::json!({
                            "key": key.to_owned(),
                            "value": err.to_string(),
                            "group": group,
                        });
                view.push(Entry {
                    timestamp: timestamp.to_owned(),
                    origin: "meta_data_errors".to_string(),
                    value: EntryValue::Value(serde_json::json!({
                                "info": format!("{}: {}", key,err.to_string()),
                                "where": filter,
                            }))
                });
            }
        }
    }
    view
}
