use crate::utils::entry::*;
use chrono::Utc;
use std::collections::HashMap;
pub mod socket;

pub fn notify_sled_db(db: &sled::Db, notification: CosmosRustServerValue) {
    //db.insert(notification.key(), notification.value()).ok();
    match notification {
        CosmosRustServerValue::UserMetaData(_) => {
            db.insert(notification.key(), notification.value()).ok();
        }
        CosmosRustServerValue::Notify(_) => {
            db.insert(notification.key(), notification.value()).ok();
        }
        CosmosRustServerValue::Notification(n) => {
            let fields: Option<Option<Vec<String>>> = n.get_query().get("fields").map(|x| {
                x.as_array().map(|yy| {
                    yy.iter()
                        .map(|y| y.as_str().unwrap_or("").to_string())
                        .collect::<Vec<String>>()
                })
            });

            match fields {
                Some(Some(fields)) => {
                    let mut field_list: Vec<HashMap<String, String>> = Vec::new();

                    for i in 0..n.entries.len() {
                        let mut m: HashMap<String, String> = HashMap::new();
                        for field in fields.iter() {
                            if let Some(val) = n.entries[i].try_get(field) {
                                if let Some(summary_text) = val.as_str() {
                                    m.insert(field.to_string(), summary_text.to_string());
                                }
                            }
                        }
                        field_list.push(m);
                    }
                    let mut msg_1: Vec<String> = field_list
                        .iter()
                        .map(|x| x.get("summary"))
                        .filter(|x| x.is_some())
                        .map(|x| x.unwrap().to_owned())
                        .collect();
                    let mut msg_2 = field_list
                        .iter()
                        .map(|x| {
                            match (x.get("key"), x.get("value")) {
                                (Some(key), Some(value)) => {
                                    return Some((key, value));
                                }
                                _ => {
                                    return None;
                                }
                            };
                        })
                        .filter(|x| x.is_some())
                        .map(|x| x.unwrap())
                        .map(|y| format!("Key: {}\nValue: {}", y.0, y.1))
                        .collect();

                    let mut msg: Vec<String> = Vec::new();
                    msg.append(&mut msg_1);
                    msg.append(&mut msg_2);

                    for user_hash in n.user_list {
                        let notify = CosmosRustServerValue::Notify(Notify {
                            timestamp: Utc::now().timestamp(),
                            msg: msg.to_owned(),
                            user_hash: user_hash,
                        });
                        db.insert(notify.key(), notify.value()).ok();
                    }
                }
                _ => {}
            };
        }
    }
}
