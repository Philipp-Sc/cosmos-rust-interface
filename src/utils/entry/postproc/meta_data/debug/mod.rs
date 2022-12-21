use crate::utils::entry::*;
use crate::utils::response::ResponseResult;
use std::collections::HashMap;

pub fn debug(maybes: impl Iterator<Item = (String,Maybe<ResponseResult>)>) -> Vec<CosmosRustBotValue> {
    let mut view: Vec<CosmosRustBotValue> = Vec::new();

    for (key, value) in maybes {
        match value {
            Maybe {
                data: Ok(resolved),
                timestamp,
            } => {
                view.push(CosmosRustBotValue::Entry(Entry::Value(Value {
                    timestamp: timestamp.to_owned(),
                    origin: "task_meta_data_debug".to_string(),
                    custom_data: CustomData::Debug(Debug{ key: format!("{}", key), value: format!("{:?}", resolved) }),
                    imperative: ValueImperative::Notify
                })));
            }
            Maybe { data: Err(_), .. } => {}
        }
    }
    CosmosRustBotValue::add_membership(&mut view, None, "task_meta_data_debug");
    view
}
