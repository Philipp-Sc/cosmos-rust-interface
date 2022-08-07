use std::collections::HashMap;
use crate::ResponseResult;
use crate::utils::postproc::{Maybe, Entry};


pub fn logs(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<Entry> {
    let mut view: Vec<Entry> = Vec::new();

    for (key, value) in maybes {
        match value {
            Maybe { data: Ok(resolved), timestamp } => {
                match resolved {
                    ResponseResult::LogEntry(text) => {
                        view.push(Entry {
                            timestamp: timestamp.to_owned(),
                            key: key.to_owned(),
                            prefix: None,
                            value: text.to_owned(),
                            suffix: None,
                            index: None,
                            group: Some("[Logs]".to_string()),
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