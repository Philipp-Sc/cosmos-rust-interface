use std::collections::HashMap;
use crate::utils::entry::{Maybe, Entry, EntryValue};
use crate::utils::response::ResponseResult;


pub fn logs(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<Entry> {
    let mut view: Vec<Entry> = Vec::new();

    for (key, value) in maybes {
        match value {
            Maybe { data: Ok(resolved), timestamp } => {
                match resolved {
                    ResponseResult::LogEntry(text) => {
                        let filter = serde_json::json!({
                            "key": key.to_owned(),
                            "value": text.to_owned(),
                            "group": "logs"
                        });
                        view.push(Entry {
                            timestamp: timestamp.to_owned(),
                            origin: "meta_data_logs".to_string(),
                            value: EntryValue::Value(serde_json::json!({
                                "info": format!("{}: {}", key,text),
                                "where": filter,
                            }))
                        });
                    }
                    _ => {}
                }
            }
            Maybe { data: Err(_failed), .. } => {}
        }
    }

    view
}