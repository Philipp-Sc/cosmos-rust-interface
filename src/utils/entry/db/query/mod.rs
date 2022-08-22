use std::collections::HashMap;
use crate::utils::entry::{Entry, EntryValue};

pub fn query_entries(entries: &Vec<Entry>, filter: HashMap<String, String>, order_by: String, limit: usize) -> Vec<&Entry> {
    let mut result: Vec<&Entry> = entries.iter().filter(|item| {
        if let EntryValue::Value(ref val) = item.value {
            if val.get("where").is_some(){
                if let Some(filter_options) = val.get("where").unwrap().as_object() {
                    let res: bool = filter.iter().map(|(k,v)| {
                        filter_options.contains_key(k) && (filter_options.get(k).unwrap() == &serde_json::json!(v) || v == "any")
                    }).fold(true,|x, y| {x && y});
                    return  res;
                }
            }
        }
        return false
    }).collect();
    result.sort_by(|a, b| {
        match (a,b) {
            (Entry{value: EntryValue::Value(x), ..},Entry{ value: EntryValue::Value(y),..}) => {
                let v = serde_json::json!({});
                let xx = match &x["order_by"] {
                    serde_json::Value::Object(obj) => {
                        match &obj[&order_by] {
                            serde_json::Value::Number(val)  => {
                                val.as_u64().unwrap_or(0)
                            }
                            _ => { 0 as u64}
                        }
                    }
                    serde_json::Value::Null | _ => {
                        0 as u64
                    }
                };

                let yy = match &y["order_by"] {
                    serde_json::Value::Object(obj) => {
                        match &obj[&order_by] {
                            serde_json::Value::Number(val) => {
                                val.as_u64().unwrap_or(0)
                            }
                            serde_json::Value::Null | _ => { 0 as u64}
                        }
                    }
                    serde_json::Value::Null | _ => {
                        0 as u64
                    }
                };

                xx.cmp(&yy)
            },
            _ => {
                panic!()
            }
        }
    });
   result.into_iter().rev().take(limit).collect()
}