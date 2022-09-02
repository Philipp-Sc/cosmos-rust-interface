use std::collections::HashMap;
use crate::utils::entry::*;
use crate::utils::response::ResponseResult;


pub fn logs(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<CosmosRustBotValue> {
    let mut view: Vec<CosmosRustBotValue> = Vec::new();

    for (key, value) in maybes {
        match value {
            Maybe { data: Ok(resolved), timestamp } => {
                match resolved {
                    ResponseResult::LogEntry(text) => {
                        view.push(CosmosRustBotValue::Entry(Entry::Log(Log {
                            timestamp: timestamp.to_owned(),
                            origin: "meta_data_logs".to_string(),
                            key: key.to_owned(),
                            value: text.to_owned(),
                            summary: format!("{}: {}", key,text),
                        })));
                    }
                    _ => {}
                }
            }
            Maybe { data: Err(_failed), .. } => {}
        }
    }
    CosmosRustBotValue::add_membership(&mut view,None,"meta_data_logs");
    view
}