use std::collections::HashMap;
use crate::utils::response::{ResponseResult};
use crate::utils::entry::*;

pub fn debug(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<CosmosRustBotValue> {
    let mut view: Vec<CosmosRustBotValue> = Vec::new();

    for (key,value) in maybes {
        match value {
            Maybe { data: Ok(resolved), timestamp } => {
                view.push(CosmosRustBotValue::Entry(Entry::Debug(Debug {
                    timestamp: timestamp.to_owned(),
                    origin: "meta_data_debug".to_string(),
                    key: format!("{}",key),
                    value: format!("{:?}",resolved),
                })));
            }
            Maybe { data: Err(_), .. } => {}
        }
    }
    CosmosRustBotValue::add_membership(&mut view,None,"meta_data_debug");
    view
}
