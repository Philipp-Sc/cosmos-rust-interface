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
            let insert_notify = |db: &sled::Db, msg: Vec<String>, buttons: Vec<Vec<Vec<(String,String)>>>, user_hash: u64| {
                let notify = CosmosRustServerValue::Notify(Notify {
                    timestamp: Utc::now().timestamp(),
                    msg,
                    buttons,
                    user_hash,
                });
                db.insert(notify.key(), TryInto::<Vec<u8>>::try_into(notify).unwrap()).ok();
            };

            match n.query.query_part {
                QueryPart::SubscriptionsQueryPart(_query_part) => {
                    if let Some(user_hash) = n.query.settings_part.user_hash {
                        if n.entries.is_empty() {
                            insert_notify(db, vec!["You have no subscriptions registered.".to_string()], vec![], user_hash);
                        } else {
                            for entry in n.entries {
                                match entry {
                                    CosmosRustBotValue::Subscription(sub) => {
                                        match sub.query {
                                            QueryPart::SubscriptionsQueryPart(_) => {},
                                            QueryPart::EntriesQueryPart(query_part) => {
                                                let command = format!("/{}",query_part.message.replace(" ", "_"));
                                                insert_notify(db, vec![command.to_owned()], vec![vec![vec![("Unsubscribe".to_string(),format!("{}_unsubscribe",command))]]], user_hash);
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
                            insert_notify(db, vec!["Subscribed".to_string()], vec![], user_hash);
                            return;
                        } else if n.query.settings_part.unsubscribe.unwrap_or(false) {
                            insert_notify(db, vec!["Unsubscribed".to_string()], vec![], user_hash);
                            return;
                        } else if n.entries.is_empty() {
                            insert_notify(db, vec!["Empty".to_string()], vec![], user_hash);
                            return;
                        }
                    }
                    if n.entries.is_empty() {
                        for user_hash in n.user_list.into_iter() {
                            insert_notify(db, vec!["Empty".to_string()], vec![], user_hash);
                        }
                    } else {


                        let mut msg: Vec<String> = Vec::new();
                        let mut buttons = Vec::new();

                        for i in 0..n.entries.len() {

                            match &n.entries[i] {
                                CosmosRustBotValue::Index(_) => {}
                                CosmosRustBotValue::Entry(Entry::Value(Value{ timestamp: _, origin: _, custom_data })) => {
                                    msg.push(custom_data.display(&query_part.display));

                                    if query_part.message.contains("gov prpsl") {

                                        let mut navigation = Vec::new();
                                        let mut navigation_row = Vec::new();

                                        if &query_part.display == "default" {

                                            if let Some(command) = custom_data.command("status"){
                                                navigation_row.push(
                                                    ("Status".to_string(), command),
                                                );
                                            }

                                            if let Some(command) = custom_data.command("summary"){
                                                navigation_row.push(
                                                    ("Summary".to_string(), command),
                                                );
                                            }

                                            if let Some(command) = custom_data.command("content"){
                                                navigation_row.push(
                                                    ("Content".to_string(), command),
                                                );
                                            }
                                            navigation.push(navigation_row);

                                            if let Some(link) = custom_data.view_in_browser() {
                                                navigation.push(vec![
                                                    ("View in Browser".to_string(), link),
                                                ]);
                                            }

                                            buttons.push(navigation);
                                        }else if &query_part.display == "status" {

                                            /*
                                            navigation.push(vec![
                                                ("Tally".to_string(), format!("{}", query_part.message)),
                                            ]);
                                            navigation.push(vec![
                                                ("Sentiment".to_string(), format!("{}", query_part.message)),
                                            ]);
                                            */

                                            // who voted how?

                                        }else if &query_part.display == "summary" {

                                        }else if &query_part.display == "content" {

                                        }




                                    }else{
                                        buttons.push(vec![]);
                                    }
                                }
                                CosmosRustBotValue::Subscription(_) => {}
                            }
                        }

                        if let Some(user_hash) = n.query.settings_part.user_hash {
                            insert_notify(db, msg, buttons, user_hash);
                        } else {
                            for user_hash in n.user_list {
                                insert_notify(db, msg.to_owned(), buttons.clone(), user_hash);
                            }
                        }

                    }
                }
            };
        }
    };
}
