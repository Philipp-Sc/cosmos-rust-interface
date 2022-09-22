use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[cfg(feature = "postproc")]
pub mod postproc;

#[cfg(feature = "db")]
pub mod db;

#[derive(Debug)]
pub struct Maybe<T> {
    pub data: anyhow::Result<T>,
    pub timestamp: i64,
}

impl<T: Clone> Clone for Maybe<T> {
    fn clone(&self) -> Maybe<T> {
        match self {
            Maybe {
                data: Err(err),
                timestamp,
            } => Maybe {
                data: Err(anyhow::anyhow!(err.to_string())),
                timestamp: *timestamp,
            },
            Maybe {
                data: Ok(value),
                timestamp,
            } => Maybe {
                data: Ok(value.clone()),
                timestamp: *timestamp,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Value {
    pub timestamp: i64,
    pub origin: String,
    pub summary: String,
    pub custom_data: String,
    //..
}
impl Value {
    pub fn new(
        timestamp: i64,
        origin: String,
        summary: String,
        custom_data: serde_json::Value,
    ) -> Value {
        Value {
            timestamp: timestamp,
            origin: origin,
            summary: summary,
            custom_data: custom_data.to_string(),
        }
    }
    pub fn custom_data(&self) -> serde_json::Value {
        serde_json::from_str(&self.custom_data.as_str()).unwrap()
    }
}
impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //self.timestamp.hash(state);
        self.origin.hash(state);
        self.summary.hash(state);
        self.custom_data.to_string().hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MetaData {
    pub index: i32,
    pub timestamp: i64,
    pub origin: String,
    pub kind: String,
    pub state: String,
    pub value: String,
    pub summary: String,
    //..
}
impl Hash for MetaData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //self.timestamp.hash(state);
        self.index.hash(state);
        self.origin.hash(state);
        self.kind.hash(state);
        self.state.hash(state);
        self.value.hash(state);
        self.summary.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Debug {
    pub timestamp: i64,
    pub origin: String,
    pub key: String,
    pub value: String,
    //..
}
impl Hash for Debug {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //self.timestamp.hash(state);
        self.origin.hash(state);
        self.key.hash(state);
        self.value.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Error {
    pub timestamp: i64,
    pub origin: String,
    pub key: String,
    pub value: String,
    pub summary: String,
    pub kind: String,
    //..
}
impl Hash for Error {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //self.timestamp.hash(state);
        self.origin.hash(state);
        self.key.hash(state);
        self.value.hash(state);
        self.summary.hash(state);
        self.kind.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Log {
    pub timestamp: i64,
    pub origin: String,
    pub key: String,
    pub value: String,
    pub summary: String,
    //..
}
impl Hash for Log {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //self.timestamp.hash(state);
        self.origin.hash(state);
        self.key.hash(state);
        self.value.hash(state);
        self.summary.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub enum Entry {
    MetaData(MetaData),
    Debug(Debug),
    Error(Error),
    Log(Log),
    Value(Value),
}

impl Entry {
    fn get_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
    pub fn get_prefix() -> Vec<u8> {
        let mut k: Vec<u8> = Vec::new();
        k.append(&mut b"entry".to_vec());
        k
    }
    pub fn get_key(&self) -> Vec<u8> {
        let mut k: Vec<u8> = Entry::get_prefix();
        k.append(&mut self.get_hash().to_ne_bytes().to_vec());
        k
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Subscription {
    pub query: QueryPart,
    pub user_list: HashSet<u64>,
    pub list: Vec<Vec<u8>>,
}
impl Subscription {
    fn get_hash(query_part: &QueryPart) -> u64 {
        let mut s = DefaultHasher::new();
        match query_part {
            QueryPart::EntriesQueryPart(q) => {
                q.hash(&mut s);
            },
            QueryPart::SubscriptionsQueryPart(q) => {
                q.hash(&mut s);
            },
        }
        s.finish()
    }
    fn calculate_hash(&self) -> u64 {
        Subscription::get_hash(&self.query)
    }
    pub fn get_prefix() -> Vec<u8> {
        let mut k: Vec<u8> = Vec::new();
        k.append(&mut b"subscription".to_vec());
        k
    }
    pub fn get_key(&self) -> Vec<u8> {
        let mut k: Vec<u8> = Subscription::get_prefix();
        k.append(&mut self.calculate_hash().to_ne_bytes().to_vec());
        k
    }
    pub fn get_key_for_entries_query(query: &EntriesQueryPart) -> Vec<u8> {
        let mut k: Vec<u8> = Subscription::get_prefix();
        let mut s = DefaultHasher::new();
        query.hash(&mut s);
        k.append(&mut s.finish().to_ne_bytes().to_vec());
        k
    }
    pub fn add_user_hash(&mut self, user_hash: u64) {
        self.user_list.insert(user_hash);
    }
    pub fn contains_user_hash(&self, user_hash: u64) -> bool {
        self.user_list.contains(&user_hash)
    }
    pub fn remove_user_hash(&mut self, user_hash: u64) -> bool {
        self.user_list.remove(&user_hash)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Notification {
    pub query: UserQuery,
    pub entries: Vec<CosmosRustBotValue>,
    pub user_list: HashSet<u64>,
}
impl Notification {
    pub fn calculate_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.query.hash(&mut s);
        s.finish()
    }
    fn get_hash(&self) -> u64 {
        Notification::calculate_hash(self)
    }
    pub fn get_prefix() -> Vec<u8> {
        let mut k: Vec<u8> = Vec::new();
        k.append(&mut b"notification".to_vec());
        k
    }
    pub fn get_key(&self) -> Vec<u8> {
        let mut k: Vec<u8> = Notification::get_prefix();
        k.append(&mut self.get_hash().to_ne_bytes().to_vec());
        k
    }
    pub fn get_key_for_query(query: &QueryPart) -> Vec<u8> {
        let mut k: Vec<u8> = Notification::get_prefix();
        let mut s = DefaultHasher::new();
        query.hash(&mut s);
        k.append(&mut s.finish().to_ne_bytes().to_vec());
        k
    }

    fn add_user_hash(&mut self, user_hash: u64) {
        self.user_list.insert(user_hash);
    }
    pub fn contains_user_hash(&self, user_hash: u64) -> bool {
        self.user_list.contains(&user_hash)
    }
    pub fn remove_user_hash(&mut self, user_hash: u64) -> bool {
        self.user_list.remove(&user_hash)
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Notify {
    pub timestamp: i64,
    pub msg: Vec<String>,
    pub user_hash: u64,
}
impl Notify {
    pub fn calculate_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.msg.hash(&mut s);
        self.timestamp.hash(&mut s);
        self.user_hash.hash(&mut s);
        s.finish()
    }
    fn get_hash(&self) -> u64 {
        Notify::calculate_hash(self)
    }
    pub fn get_prefix() -> Vec<u8> {
        let mut k: Vec<u8> = Vec::new();
        k.append(&mut b"notify".to_vec());
        k
    }
    pub fn get_key(&self) -> Vec<u8> {
        let mut k: Vec<u8> = Notify::get_prefix();
        k.append(&mut self.get_hash().to_ne_bytes().to_vec());
        k
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct UserQuery {
    pub query_part: QueryPart,
    pub settings_part: SettingsPart,
}
impl Hash for UserQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.query_part.hash(state);
        self.settings_part.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct SettingsPart {
    pub subscribe: Option<bool>,
    pub unsubscribe: Option<bool>,
    pub update_subscription: Option<bool>,
    pub user_hash: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum QueryPart {
    EntriesQueryPart(EntriesQueryPart),
    SubscriptionsQueryPart(SubscriptionsQueryPart)
}
impl Hash for QueryPart {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self {
            QueryPart::EntriesQueryPart(q) => {
                q.hash(state);
            },
            QueryPart::SubscriptionsQueryPart(q) => {
                q.hash(state);
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EntriesQueryPart {
    pub message: String,
    pub fields: Vec<String>,
    pub indices: Vec<String>,
    pub filter: HashMap<String, String>,
    pub order_by: String,
    pub limit: usize,
}
impl Hash for EntriesQueryPart {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.message.hash(state);
        self.fields.hash(state);
        self.indices.hash(state);
        let mut key_value_vector: Vec<String> = self.filter.iter().map(|(k, v)| format!("{},{}",k,v)).collect();
        key_value_vector.sort_unstable();
        key_value_vector.dedup();
        key_value_vector.join(";").hash(state);
        self.order_by.hash(state);
        self.limit.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct  SubscriptionsQueryPart {
    pub message: String,
}

impl UserQuery {
    pub fn value(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
    pub fn from(value: Vec<u8>) -> UserQuery {
        bincode::deserialize(&value[..]).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct UserMetaData {
    pub timestamp: i64,
    pub user_id: u64,
    pub user_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub language_code: Option<String>,
    pub user_chat_id: i64,
}
impl UserMetaData {
    pub fn user_hash(user_id: u64) -> u64 {
        let mut s = DefaultHasher::new();
        user_id.hash(&mut s);
        s.finish()
    }
    fn get_hash(&self) -> u64 {
        UserMetaData::user_hash(self.user_id)
    }
    pub fn get_key(&self) -> Vec<u8> {
        self.get_hash().to_ne_bytes().to_vec()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum CosmosRustServerValue {
    Notification(Notification),
    Notify(Notify),
    UserMetaData(UserMetaData),
}
impl CosmosRustServerValue {
    pub fn key(&self) -> Vec<u8> {
        match self {
            CosmosRustServerValue::Notification(entry) => entry.get_key(),
            CosmosRustServerValue::Notify(entry) => entry.get_key(),
            CosmosRustServerValue::UserMetaData(entry) => entry.get_key(),
        }
    }
    pub fn value(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
    pub fn from(value: Vec<u8>) -> CosmosRustServerValue {
        bincode::deserialize(&value[..]).unwrap()
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Index {
    // may contain members or an ordering
    pub name: String,
    pub list: Vec<Vec<u8>>,
}
impl Index {
    fn calculate_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.name.as_str().hash(&mut s);
        self.list.hash(&mut s);
        s.finish()
    }
    fn get_hash(&self) -> u64 {
        Index::calculate_hash(self)
    }
    pub fn get_prefix() -> Vec<u8> {
        let mut k: Vec<u8> = Vec::new();
        k.append(&mut b"index".to_vec());
        k
    }
    pub fn get_key(&self) -> Vec<u8> {
        let mut k: Vec<u8> = Index::get_prefix();
        k.append(&mut self.get_hash().to_ne_bytes().to_vec());
        k
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum CosmosRustBotValue {
    Index(Index),
    Entry(Entry),
    Subscription(Subscription),
}

impl CosmosRustBotValue {
    pub fn key(&self) -> Vec<u8> {
        match self {
            CosmosRustBotValue::Entry(entry) => entry.get_key(),
            CosmosRustBotValue::Index(index) => index.get_key(),
            CosmosRustBotValue::Subscription(sub) => sub.get_key(),
        }
    }
    pub fn value(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
    pub fn from(value: Vec<u8>) -> CosmosRustBotValue {
        bincode::deserialize(&value[..]).unwrap()
    }
    pub fn try_get(&self, field: &str) -> Option<serde_json::Value> {
        match self {
            CosmosRustBotValue::Entry(entry) => match entry {
                Entry::Value(val) => match field {
                    "timestamp" => Some(serde_json::json!(val.timestamp)),
                    "origin" => Some(serde_json::json!(val.origin)),
                    "summary" => Some(serde_json::json!(val.summary)),
                    &_ => val.custom_data().get(field).map(|x| x.clone()),
                },
                Entry::MetaData(val) => match field {
                    "index" => Some(serde_json::json!(val.index)),
                    "timestamp" => Some(serde_json::json!(val.timestamp)),
                    "origin" => Some(serde_json::json!(val.origin)),
                    "kind" => Some(serde_json::json!(val.kind)),
                    "state" => Some(serde_json::json!(val.state)),
                    "value" => Some(serde_json::json!(val.value)),
                    "summary" => Some(serde_json::json!(val.summary)),
                    &_ => None,
                },
                Entry::Log(val) => match field {
                    "summary" => Some(serde_json::json!(val.summary)),
                    "timestamp" => Some(serde_json::json!(val.timestamp)),
                    "origin" => Some(serde_json::json!(val.origin)),
                    &_ => None,
                },
                Entry::Error(val) => match field {
                    "summary" => Some(serde_json::json!(val.summary)),
                    "timestamp" => Some(serde_json::json!(val.timestamp)),
                    "origin" => Some(serde_json::json!(val.origin)),
                    "kind" => Some(serde_json::json!(val.kind)),
                    &_ => None,
                },
                Entry::Debug(val) => match field {
                    "key" => Some(serde_json::json!(val.key)),
                    "value" => Some(serde_json::json!(val.value)),
                    "timestamp" => Some(serde_json::json!(val.timestamp)),
                    "origin" => Some(serde_json::json!(val.origin)),
                    &_ => None,
                },
            },
            CosmosRustBotValue::Index(val) => match field {
                "name" => Some(serde_json::json!(val.name)),
                "list" => Some(serde_json::json!(val.list)),
                &_ => None,
            },
            CosmosRustBotValue::Subscription(val) => match field {
                "query" => Some(serde_json::json!(val.query)),
                "user_list" => Some(serde_json::json!(val.user_list)),
                "list" => Some(serde_json::json!(val.list)),
                &_ => None,
            },
        }
    }
    pub fn add_variants_of_memberships(view: &mut Vec<CosmosRustBotValue>, fields: Vec<&str>) {
        for field in fields {
            let variants = view
                .iter()
                .filter(|x| x.try_get(field).is_some())
                .map(|x| x.try_get(field).unwrap().as_str().unwrap().to_string())
                .collect::<HashSet<String>>();
            for variant in variants {
                let entries = view
                    .iter()
                    .filter(|x| match x.try_get(field) {
                        Some(t) => t.as_str().unwrap() == variant,
                        None => false,
                    })
                    .map(|x| x.clone())
                    .collect::<Vec<CosmosRustBotValue>>();
                let membership = CosmosRustBotValue::create_membership(
                    &entries,
                    None,
                    format!("{}_{}", field, variant.to_lowercase()).as_str(),
                );
                view.push(CosmosRustBotValue::Index(membership));
            }
        }
    }
    pub fn add_membership(entries: &mut Vec<CosmosRustBotValue>, field: Option<&str>, name: &str) {
        let index = CosmosRustBotValue::create_membership(entries, field, name);
        entries.push(CosmosRustBotValue::Index(index));
    }
    pub fn create_membership(
        entries: &Vec<CosmosRustBotValue>,
        field: Option<&str>,
        name: &str,
    ) -> Index {
        let have_field = entries
            .iter()
            .map(|x| {
                if let Some(f) = field {
                    (x.key(), x.try_get(f))
                } else {
                    (x.key(), Some(serde_json::Value::Null))
                }
            })
            .filter(|(_, x)| x.is_some())
            .map(|(key, _)| key)
            .collect::<Vec<Vec<u8>>>();
        Index {
            name: name.to_string(),
            list: have_field,
        }
    }
    pub fn add_index(entries: &mut Vec<CosmosRustBotValue>, field: &str, name: &str) {
        let index = CosmosRustBotValue::create_index(entries, field, name);
        entries.push(CosmosRustBotValue::Index(index));
    }
    pub fn create_index(entries: &Vec<CosmosRustBotValue>, field: &str, name: &str) -> Index {
        let mut have_field = entries
            .iter()
            .map(|x| (x.key(), x.try_get(field)))
            .filter(|(_, x)| x.is_some())
            .map(|(key, x)| (key, x.unwrap()))
            .collect::<Vec<(Vec<u8>, serde_json::Value)>>();
        have_field.sort_by(|(_, first), (_, second)| match (first, second) {
            (serde_json::Value::String(f), serde_json::Value::String(s)) => {
                match (f.parse::<u64>(),s.parse::<u64>()) {
                    (Ok(ff), Ok(ss)) => {
                        ff.cmp(&ss)
                    },
                    _ => {
                        match (f.parse::<f64>(),s.parse::<f64>()) {
                            (Ok(ff), Ok(ss)) => {
                                ff.total_cmp(&ss)
                            },
                            _ => {
                                f.cmp(s)
                            }
                        }
                    }
                }
            },
            (serde_json::Value::Number(f), serde_json::Value::Number(s)) => {
                if f.is_u64() && s.is_u64() {
                    f.as_u64().unwrap().cmp(&s.as_u64().unwrap())
                } else if f.is_i64() && s.is_i64() {
                    f.as_i64().unwrap().cmp(&s.as_i64().unwrap())
                } else if f.is_f64() && s.is_f64() {
                    f.as_f64().unwrap().total_cmp(&s.as_f64().unwrap())
                } else {
                    Ordering::Equal
                }
            }
            _ => {
                match (first.to_string().parse::<u64>(),second.to_string().parse::<u64>()) {
                    (Ok(ff), Ok(ss)) => {
                        ff.cmp(&ss)
                    },
                    _ => {
                        match (first.to_string().parse::<f64>(),second.to_string().parse::<f64>()) {
                            (Ok(ff), Ok(ss)) => {
                                ff.total_cmp(&ss)
                            },
                            _ => {
                                first.to_string().cmp(&second.to_string())
                            }
                        }
                    }
                }
            },
        });
        Index {
            name: name.to_string(),
            list: have_field.into_iter().rev().map(|(key, _)| key).collect(),
        }
    }
}
