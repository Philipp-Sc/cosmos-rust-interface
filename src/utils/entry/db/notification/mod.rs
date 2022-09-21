use crate::utils::entry::*;
use chrono::Utc;
use std::collections::HashMap;

pub mod socket;

pub fn notify_sled_db(db: &sled::Db, notification: CosmosRustServerValue) {
    match notification {
        CosmosRustServerValue::UserMetaData(_) => {
            db.insert(notification.key(), notification.value()).ok();
        }
        CosmosRustServerValue::Notify(_) => {
            db.insert(notification.key(), notification.value()).ok();
        }
        CosmosRustServerValue::Notification(n) => {

            let query = n.get_query();

            let handler = query
                .get("handler")
                .map(|x| x.as_str());


            let user_hash = query
                .get("user_id")
                .map(|x| Subscription::user_hash(x.as_u64().unwrap_or(0)));


            let unsubscribe = query
                .get("unsubscribe")
                .map(|x| x.as_bool().unwrap_or(false))
                .unwrap_or(false);

            let subscribe = query
                .get("subscribe")
                .map(|x| x.as_bool().unwrap_or(false))
                .unwrap_or(false);

            match handler {
                Some(Some("query_subscriptions")) => {
                    if n.entries.is_empty(){
                        let notify = CosmosRustServerValue::Notify(Notify {
                            timestamp: Utc::now().timestamp(),
                            msg: vec!["You have no subscriptions registered.".to_string()],
                            user_hash: user_hash.unwrap(),
                        });
                        db.insert(notify.key(), notify.value()).ok();
                    }else {
                        for entry in n.entries {
                            match entry {
                                CosmosRustBotValue::Subscription(sub) => {
                                    let notify = CosmosRustServerValue::Notify(Notify {
                                        timestamp: Utc::now().timestamp(),
                                        msg: vec![sub.get_query().get("message").map(|x| format!("/{}",x.as_str().unwrap_or("Error: Could not parse message!")).replace(" ","_")).unwrap_or("Error: No message defined!".to_string())],
                                        user_hash: user_hash.unwrap(),
                                    });
                                    db.insert(notify.key(), notify.value()).ok();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Some(Some("query_entries")) => {

                    if subscribe {
                        let notify = CosmosRustServerValue::Notify(Notify {
                            timestamp: Utc::now().timestamp(),
                            msg: vec!["Subscribed".to_string()],
                            user_hash: user_hash.unwrap(),
                        });
                        db.insert(notify.key(), notify.value()).ok();
                    } else if unsubscribe {
                        let notify = CosmosRustServerValue::Notify(Notify {
                            timestamp: Utc::now().timestamp(),
                            msg: vec!["Unsubscribed".to_string()],
                            user_hash: user_hash.unwrap(),
                        });
                        db.insert(notify.key(), notify.value()).ok();
                    }else if n.entries.is_empty() {
                        let notify = CosmosRustServerValue::Notify(Notify {
                            timestamp: Utc::now().timestamp(),
                            msg: vec!["Empty".to_string()],
                            user_hash: user_hash.unwrap(),
                        });
                        db.insert(notify.key(), notify.value()).ok();
                    }else{

                    let fields: Option<Option<Vec<String>>> = query.get("fields").map(|x| {
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
                _ => {}
            };
        }
    };
}
