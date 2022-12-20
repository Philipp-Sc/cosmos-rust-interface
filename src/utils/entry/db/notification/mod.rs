use crate::utils::entry::*;
use chrono::Utc;
use std::collections::HashMap;

pub mod socket;

// TODO: if a user subscribes or unsubscribes or user data gets passed then write that data to a import export database
// each time the tg bot is restarted it reloads the data.
// make a special command to write out (export) all current subscriptions by each user. into json. call it snapshot.
// make a special command to write out (export) all user meta data into json.
// make a special command to load both json files.
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
                QueryPart::SubscriptionsQueryPart(subscription_query_part) => {
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

                                                let action = if subscription_query_part.message.contains("unsubscribe"){
                                                    "Subscribe".to_string()
                                                }else{
                                                    "Unsubscribe".to_string()
                                                };

                                                let command = format!("/{}",query_part.message.replace(" ", "_"));
                                                insert_notify(db, vec![command.to_owned()], vec![vec![vec![(action.to_owned(),format!("{} {}",query_part.message,action.to_lowercase()))]]], user_hash);
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

                    let command = format!("/{}",query_part.message.replace(" ", "_"));

                    if let Some(user_hash) = n.query.settings_part.user_hash {
                        if n.query.settings_part.subscribe.unwrap_or(false) {
                            insert_notify(db, vec![format!("Subscribed\n{}", command)], vec![], user_hash);
                            return;
                        } else if n.query.settings_part.unsubscribe.unwrap_or(false) {
                            insert_notify(db, vec![format!("Unsubscribed\n{}", command)], vec![], user_hash);
                            return;
                        } else if n.entries.is_empty() {
                            insert_notify(db, vec![format!("Empty result set\n{}", command)], vec![], user_hash);
                            return;
                        }
                    }
                    if n.entries.is_empty() {
                        for user_hash in n.user_list.into_iter() {
                            insert_notify(db, vec![format!("Empty result set\n{}", command)], vec![], user_hash);
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
                                        let mut navigation_row2 = Vec::new();

                                        if &query_part.display == "default" {

                                            if let Some(command) = custom_data.command("status"){
                                                navigation_row.push(
                                                    ("📊 Status".to_string(), command),
                                                );
                                            }

                                            if let Some(command) = custom_data.command("content"){
                                                navigation_row.push(
                                                    ("🏛 ️Proposal".to_string(), command),
                                                );
                                            }

                                            if let Some(link) = custom_data.view_in_browser() {
                                                navigation_row2.push(
                                                    ("Open in Browser".to_string(), link),
                                                );
                                            }

                                            if let Some(command) = custom_data.command("briefing0"){
                                                navigation_row2.push(
                                                    ("⚡ Start Briefing".to_string(), command),
                                                );
                                            }



                                            navigation.push(navigation_row);
                                            navigation.push(navigation_row2);

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

                                        }else if &query_part.display == "briefing0" {

/*
                                            "🛠️ Feasibility and technical viability"
                                            "💸 Economic impact"
                                            "⚖️ Legal and regulatory compliance"
                                            "🌿 Long-term sustainability"
                                            "🔎 Transparency & Accountability"
                                            "👥 Community Support"
                                            "⚠️ Risks"
                                            "🎉 Benefits"
                                            "🤔 Recommendations or advice"

 */

                                            if let Some(command) = custom_data.command("briefing1"){
                                                navigation.push(
                                                    vec![("🛠️ Feasibility and technical viability".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing2"){
                                                navigation.push(
                                                    vec![("💸 Economic impact".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing3"){
                                                navigation.push(
                                                    vec![("⚖️ Legal and regulatory compliance".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing4"){
                                                navigation.push(
                                                    vec![("🌿 Long-term sustainability".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing5"){
                                                navigation.push(
                                                    vec![("🔎 Transparency & Accountability".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing6"){
                                                navigation.push(
                                                    vec![("👥 Community Support".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing7"){
                                                navigation.push(
                                                    vec![("⚠️ Risks".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing8"){
                                                navigation.push(
                                                    vec![("🎉 Benefits".to_string(), command)],
                                                );
                                            }
                                            if let Some(command) = custom_data.command("briefing9"){
                                                navigation.push(
                                                    vec![("🤔 Recommendations or advice".to_string(), command)],
                                                );
                                            }


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
