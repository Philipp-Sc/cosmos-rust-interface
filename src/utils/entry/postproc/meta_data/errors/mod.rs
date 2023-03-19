use crate::utils::entry::*;
use crate::utils::response::ResponseResult;
use std::collections::HashMap;
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};

pub fn errors(task_store: &TaskMemoryStore) -> Vec<CosmosRustBotValue> {
    let mut view: Vec<CosmosRustBotValue> = Vec::new();

    for (key, value) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::Get) {
        match value {
            Maybe {
                data: Ok(_resolved),
                ..
            } => {}
            Maybe {
                data: Err(err),
                timestamp,
            } => {
                let kind = match err.to_string().as_str() {
                    "Error: Not yet resolved!" => "unresolved",
                    "Error: Entry reserved!" => "reserved",
                    &_ => "error",
                };
                view.push(CosmosRustBotValue::Entry(Entry::Value(Value {
                    timestamp: timestamp.to_owned(),
                    origin: "task_meta_data_errors".to_string(),
                    custom_data: CustomData::Error(Error{
                        key: key.to_owned(),
                        value: err.to_string(),
                        summary: format!("[{}] - {}: {}", kind, key, err.to_string()),
                        kind: kind.to_owned(),
                    }),
                    imperative: ValueImperative::Notify
                })));
            }
        }
    }
    CosmosRustBotValue::add_membership(&mut view, None, "task_meta_data_errors");
    CosmosRustBotValue::add_variants_of_memberships(&mut view, vec!["kind"]);
    view
}
