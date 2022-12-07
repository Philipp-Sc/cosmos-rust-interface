use crate::utils::entry::*;
use chrono::Utc;
use std::collections::HashMap;

pub mod socket;

pub fn notify_sled_db(db: &sled::Db, notification: CosmosRustServerValue) {
    match notification {
        CosmosRustServerValue::UserMetaData(_) => {
            db.insert(notification.key(), TryInto::<Vec<u8>>::try_into(notification).unwrap()).ok();
        }
        CosmosRustServerValue::Notify(_) => {
            db.insert(notification.key(), TryInto::<Vec<u8>>::try_into(notification).unwrap()).ok();
        }
        CosmosRustServerValue::Notification(n) => {
            match n.query.query_part {
                QueryPart::SubscriptionsQueryPart(_query_part) => {
                    if let Some(user_hash) = n.query.settings_part.user_hash {
                        if n.entries.is_empty() {
                            let notify = CosmosRustServerValue::Notify(Notify {
                                timestamp: Utc::now().timestamp(),
                                msg: vec!["You have no subscriptions registered.".to_string()],
                                buttons: vec![],
                                user_hash,
                            });
                            db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                        } else {
                            for entry in n.entries {
                                match entry {
                                    CosmosRustBotValue::Subscription(sub) => {
                                        match sub.query {
                                            QueryPart::SubscriptionsQueryPart(_) => {},
                                            QueryPart::EntriesQueryPart(query_part) => {
                                                let notify = CosmosRustServerValue::Notify(Notify {
                                                    timestamp: Utc::now().timestamp(),
                                                    msg: vec![format!("/{}",query_part.message.replace(" ", "_"))],
                                                    buttons: vec![],
                                                    user_hash,
                                                });
                                                db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                                            },
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                QueryPart::EntriesQueryPart(query_part) => {
                    if let Some(user_hash) = n.query.settings_part.user_hash {
                        if n.query.settings_part.subscribe.unwrap_or(false) {
                            let notify = CosmosRustServerValue::Notify(Notify {
                                timestamp: Utc::now().timestamp(),
                                msg: vec!["Subscribed".to_string()],
                                buttons: vec![],
                                user_hash,
                            });
                            db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                            return;
                        } else if n.query.settings_part.unsubscribe.unwrap_or(false) {
                            let notify = CosmosRustServerValue::Notify(Notify {
                                timestamp: Utc::now().timestamp(),
                                msg: vec!["Unsubscribed".to_string()],
                                buttons: vec![],
                                user_hash,
                            });
                            db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                            return;
                        } else if n.entries.is_empty() {
                            let notify = CosmosRustServerValue::Notify(Notify {
                                timestamp: Utc::now().timestamp(),
                                msg: vec!["Empty".to_string()],
                                buttons: vec![],
                                user_hash,
                            });
                            db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                            return;
                        }
                    }
                    if n.entries.is_empty() {
                        for user_hash in n.user_list.into_iter() {
                            let notify = CosmosRustServerValue::Notify(Notify {
                                timestamp: Utc::now().timestamp(),
                                msg: vec!["Empty".to_string()],
                                buttons: vec![],
                                user_hash,
                            });
                            db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                        }
                    } else {
                        let mut field_list: Vec<HashMap<String, String>> = Vec::new();

                        for i in 0..n.entries.len() {
                            let mut m: HashMap<String, String> = HashMap::new();
                            for field in &query_part.fields {
                                if let Some(val) = n.entries[i].try_get(&field) {
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

                        if let Some(user_hash) = n.query.settings_part.user_hash {
                            let notify = CosmosRustServerValue::Notify(Notify {
                                timestamp: Utc::now().timestamp(),
                                msg: msg.to_owned(),
                                buttons: vec![],
                                user_hash,
                            });
                            db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                        }else{
                            for user_hash in n.user_list {
                                let notify = CosmosRustServerValue::Notify(Notify {
                                    timestamp: Utc::now().timestamp(),
                                    msg: msg.to_owned(),
                                    buttons: vec![],
                                    user_hash,
                                });
                                db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
                            }
                        }
                    }
                }
            };
        }
    };
}
