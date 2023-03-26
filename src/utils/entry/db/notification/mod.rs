use crate::utils::entry::*;
use chrono::Utc;
use std::collections::HashMap;
use std::iter::FilterMap;

pub mod socket;

// TODO: the whole thing needs to be refactored into a NotificationStore struct.

const CRB_USER_META_DATA_STORE_JSON: &str = "./tmp/cosmos_rust_telegram_bot_user_meta_data.json";

pub fn get_user_meta_data(db: &sled::Db) -> impl Iterator<Item = UserMetaData> {

    db.iter().values().filter_map(|x| {
        if let Ok(value) = x {
            if let CosmosRustServerValue::UserMetaData(user_meta_data) =
            CosmosRustServerValue::try_from(value.to_vec()).unwrap()
            {
                return Some(user_meta_data);
            }
        }
        return None;
    })
}

pub fn export_user_meta_data(db: &sled::Db, path: &str){
    let json = serde_json::json!(get_user_meta_data(db).collect::<Vec<UserMetaData>>());
    if let Ok(serialized) = serde_json::to_string_pretty(&json) {
        std::fs::write(path, serialized).ok();
    }
}

// TODO: implement when I broke things.
pub fn import_user_meta_data(db: &sled::Db, path: &str){
    if let Ok(contents) = std::fs::read_to_string(path){
        if let Ok(user_meta_data) = serde_json::from_str::<Vec<UserMetaData>>(&contents){
            for data in user_meta_data {
                let item = CosmosRustServerValue::UserMetaData(data);
                let key = item.key();
                let value: Vec<u8> = item.try_into().unwrap();
                db.insert(&key, value).ok();
            }
        }
    }
}

pub fn notify_sled_db(db: &sled::Db, notification: CosmosRustServerValue) {
    match notification {
        CosmosRustServerValue::UserMetaData(_) => {
            db.insert(notification.key(), TryInto::<Vec<u8>>::try_into(notification).unwrap()).ok();
            // every time a user writes to the bot. TODO: improve this.
            export_user_meta_data(db,CRB_USER_META_DATA_STORE_JSON);
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
                                            QueryPart::RegisterQueryPart(_) => {},
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
                                CosmosRustBotValue::Entry(Entry::Value(Value{ timestamp: _, origin: _, custom_data, imperative: _ })) => {
                                    msg.push(custom_data.display(&query_part.display));

                                    if query_part.message.contains("gov prpsl") {
                                        let mut navigation = Vec::new();
                                        let mut navigation_row = Vec::new();
                                        let mut navigation_row2 = Vec::new();

                                        if &query_part.display == "default" {

                                            /*
                                            if let Some(command) = custom_data.command("status") {
                                                navigation_row.push(
                                                    ("📊 Status".to_string(), command),
                                                );
                                            }

                                            if let Some(command) = custom_data.command("content") {
                                                navigation_row.push(
                                                    ("🏛 ️Proposal".to_string(), command),
                                                );
                                            }

                                            if let Some(command) = custom_data.command("briefing0") {
                                                navigation_row.push(
                                                    ("⚡ Start Briefing".to_string(), command),
                                                );
                                            }
                                            */

                                            if let Some(link) = custom_data.view_in_browser() {
                                                navigation_row2.push(
                                                    ("Open in Browser".to_string(), link),
                                                );
                                            }


                                            navigation.push(navigation_row);
                                            navigation.push(navigation_row2);

                                            buttons.push(navigation);
                                        } else if &query_part.display == "status" {
                                            /*
                                            navigation.push(vec![
                                                ("Tally".to_string(), format!("{}", query_part.message)),
                                            ]);
                                            navigation.push(vec![
                                                ("Sentiment".to_string(), format!("{}", query_part.message)),
                                            ]);
                                            */
 

                                        } else if &query_part.display == "briefing0" { // summary
                                            if let Some(command) = custom_data.command("briefing1"){
                                                navigation.push(
                                                    vec![("❓ What problem is it solving?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);
                                        } else if &query_part.display == "briefing1" { // why is it important?
                                            if let Some(command) = custom_data.command("briefing2"){
                                                navigation.push(
                                                    vec![("⚠️ What are the risks or downsides?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);
                                        }  else if &query_part.display == "briefing2" { // risk and downsides

                                            if let Some(command) = custom_data.command("briefing3"){
                                                navigation.push(
                                                    vec![("🛠️ Is this proposal feasible and viable?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);

                                        }  else if &query_part.display == "briefing3" {

                                            if let Some(command) = custom_data.command("briefing4"){
                                                navigation.push(
                                                    vec![("💸 What is the economic impact?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);

                                        }  else if &query_part.display == "briefing4" {

                                            if let Some(command) = custom_data.command("briefing5"){
                                                navigation.push(
                                                    vec![("⚖️ Is it legally compliant?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);

                                        }  else if &query_part.display == "briefing5" {

                                            if let Some(command) = custom_data.command("briefing6"){
                                                navigation.push(
                                                    vec![("🌿 Is it sustainable?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);

                                        }  else if &query_part.display == "briefing6" {

                                            if let Some(command) = custom_data.command("briefing7"){
                                                navigation.push(
                                                    vec![("🔎 Is it transparent and accountable?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);

                                        }  else if &query_part.display == "briefing7" {

                                            if let Some(command) = custom_data.command("briefing8"){
                                                navigation.push(
                                                    vec![("👥 Is there community support?".to_string(), command)],
                                                );
                                            }
                                            buttons.push(navigation);

                                        } else if &query_part.display == "content" {

                                        }

                                    }else{
                                        buttons.push(vec![]);
                                    }
                                }
                                CosmosRustBotValue::Subscription(_) => {}
                                CosmosRustBotValue::Registration(_) => {}
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
                QueryPart::RegisterQueryPart(_) => {}
            };
        }
    };
}
