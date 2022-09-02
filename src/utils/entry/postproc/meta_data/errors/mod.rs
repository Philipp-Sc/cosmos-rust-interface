use std::collections::HashMap;
use crate::utils::response::{ResponseResult};
use crate::utils::entry::*;


pub fn errors(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<CosmosRustBotValue> {
    let mut view: Vec<CosmosRustBotValue> = Vec::new();

    for (key,value) in maybes {
        match value {
            Maybe { data: Ok(_resolved), .. } => {}
            Maybe { data: Err(err), timestamp } => {
                let kind  = match err.to_string().as_str() {
                    "Error: Not yet resolved!" => {
                        "unresolved"
                    },
                    "Error: Entry reserved!" => {
                        "reserved"
                    },
                    &_ => {
                        "error"
                    }
                };
                view.push(CosmosRustBotValue::Entry(Entry::Error(Error {
                    timestamp: timestamp.to_owned(),
                    origin: "meta_data_errors".to_string(),
                    key: key.to_owned(),
                    value: err.to_string(),
                    summary: format!("[{}] - {}: {}",kind, key, err.to_string()),
                    kind: kind.to_owned(),
                })));
            }
        }
    }
    CosmosRustBotValue::add_membership(&mut view,None,"meta_data_errors");
    view
}
