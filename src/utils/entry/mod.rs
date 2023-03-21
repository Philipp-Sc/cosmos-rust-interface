use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet};
use std::hash::{Hash, Hasher};
use std::convert::From;

use std::{
    cmp::PartialEq,
    error::Error as StdError,
    fmt::{self, Display},
};

#[cfg(feature = "postproc")]
pub mod postproc;

#[cfg(feature = "db")]
pub mod db;

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct Maybe<T> {
    pub data: Result<T,MaybeError>,
    pub timestamp: i64,
}

impl <T: for<'a> Deserialize<'a>>TryFrom<Vec<u8>> for Maybe<T> {
    type Error = anyhow::Error;
    fn try_from(item: Vec<u8>) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(&item[..])?)
    }
}

impl <T: Serialize>TryFrom<Maybe<T>> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(item: Maybe<T>) -> anyhow::Result<Self> {
        Ok(bincode::serialize(&item)?)
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum MaybeError {
    NotYetResolved(String),
    KeyDoesNotExist(String),
    EntryReserved(String),
    AnyhowError(String),
}

impl StdError for MaybeError {}

impl Display for MaybeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> std::result::Result<(), fmt::Error> {
        use self::MaybeError::*;

        match *self {
            NotYetResolved(ref key) => write!(f, "Error: Not yet resolved!: {}", key),
            KeyDoesNotExist(ref key) => write!(f, "Error: Key does not exist!: {}", key),
            EntryReserved(ref key) => write!(f, "Error: Entry reserved!: {}", key),
            AnyhowError(ref text) => write!(f, "{}", text),
        }
    }
}

#[derive(Serialize,Deserialize,Debug, Clone,Hash,PartialEq)]
pub enum CustomData {
    MetaData(MetaData),
    ProposalData(ProposalData),
    Debug(Debug),
    Error(Error),
    Log(Log)
}

impl CustomData {
    fn get(&self, field: &str) -> serde_json::Value {
        match &self {
            CustomData::ProposalData(o) => {
                o.get(field)
            },
            CustomData::MetaData(o) => {
                o.get(field)
            }
            CustomData::Debug(o) => {
                o.get(field)
            }
            CustomData::Error(o) => {
                o.get(field)
            }
            CustomData::Log(o) => {
                o.get(field)
            }
        }
    }
    fn default_display(&self) -> String {
        match &self {
            CustomData::ProposalData(o) => {
                o.proposal_clickbait.to_owned()
            },
            CustomData::MetaData(o) => {
                o.summary.to_owned()
            }
            CustomData::Debug(o) => {
                format!("{:?}",(&o.key,&o.value))
            }
            CustomData::Error(o) => {
                o.summary.to_owned()
            }
            CustomData::Log(o) => {
                o.summary.to_owned()
            }
        }
    }
    fn status_display(&self) -> String {
        match &self {
            CustomData::ProposalData(o) => {
                o.proposal_state.to_owned()
            },
            _ => {
                "Error: Can not display status for self.".to_string()
            }
        }
    }
    fn briefing_display(&self, index: usize) -> String {
        match &self {
            CustomData::ProposalData(o) => {
                o.proposal_briefings[index].to_owned()
            },
            _ => {
                "Error: Can not display summary for self.".to_string()
            }
        }
    }
    fn content_display(&self) -> String {
        match &self {
            CustomData::ProposalData(o) => {
                o.proposal_content.to_owned()
            },
            _ => {
                "Error: Can not display content for self.".to_string()
            }
        }
    }
    fn display(&self, display: &str) -> String {
        match display {
            "default" => self.default_display(),
            "status" => self.status_display(),
            "content" => self.content_display(),
            _ => {
                if display.contains("briefing") {
                    let briefing_index = display["briefing".len()..].to_string().parse::<u8>().unwrap_or(0u8);
                    self.briefing_display(briefing_index as usize)
                }else{
                    self.default_display()
                }
            },
        }
    }

    fn command(&self, display: &str) -> Option<String> {
        match &self {
            CustomData::ProposalData(o) => {
                Some(format!("gov prpsl {} {} id{}",display,o.proposal_blockchain,o.proposal_id.map(|x| x.to_string()).unwrap_or("?".to_string())))
            },
            _ => {
                None
            }
        }
    }
    fn view_in_browser(&self) -> Option<String> {
        match &self {
            CustomData::ProposalData(o) => {
                Some(o.proposal_api.to_owned())
            },
            _ => {
                None
            }
        }
    }
}

trait GetField {
    fn get(&self, field_name: &str) -> serde_json::Value where Self: serde::Serialize {
        match serde_json::to_value(&self).unwrap().get(field_name) {
            Some(value) => value.clone(),
            None => serde_json::Value::Null,
        }
    }
}

#[derive(Serialize,Deserialize,Debug, Clone,PartialEq, Hash)]
pub struct ProposalData {
    pub proposal_api: String,
    pub proposal_link: String,
    pub proposal_clickbait: String,
    pub proposal_details: String,
    pub proposal_blockchain: String,
    pub proposal_status: String,
    pub proposal_id: Option<u64>,
    pub proposal_type: Option<String>,
    pub proposal_SubmitTime: Option<i64>,
    pub proposal_DepositEndTime: Option<i64>,
    pub proposal_VotingStartTime: Option<i64>,
    pub proposal_VotingEndTime: Option<i64>,
    pub proposal_LatestTime: Option<i64>,
    pub proposal_title: String,
    pub proposal_description: String,
    pub proposal_vetoed: bool,
    pub proposal_briefings: Vec<String>,
    pub proposal_content: String,
    pub proposal_state: String,
}

impl GetField for ProposalData {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct MetaData {
    pub index: i32,
    pub kind: String,
    pub state: String,
    pub value: String,
    pub summary: String,
}

impl GetField for MetaData {}


#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct Debug {
    pub key: String,
    pub value: String,
}

impl GetField for Debug {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct Error {
    pub key: String,
    pub value: String,
    pub summary: String,
    pub kind: String,
}

impl GetField for Error {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct Log {
    pub key: String,
    pub value: String,
    pub summary: String,
}
impl GetField for Log {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ValueImperative {
    Notify,
    Update,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Value {
    pub timestamp: i64,
    pub origin: String,
    pub custom_data: CustomData,
    pub imperative: ValueImperative,
}
impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //self.timestamp.hash(state);
        self.origin.hash(state);
        self.custom_data.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub enum Entry {
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
pub enum SubscriptionAction {
    Created,
    AddUser,
    RemoveUser,
    Update,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Subscription {
    pub action: SubscriptionAction,
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
            QueryPart::RegisterQueryPart(q) => {
                q.hash(&mut s);
            }
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
pub struct Registration {
    pub token: u64,
    pub user_hash: u64,
}
impl Registration {
    pub fn get_prefix() -> Vec<u8> {
        let mut k: Vec<u8> = Vec::new();
        k.append(&mut b"registration".to_vec());
        k
    }
    pub fn get_key(&self) -> Vec<u8> {
        Registration::get_key_for_user_hash(self.user_hash)
    }
    pub fn get_key_for_user_hash(user_hash: u64) -> Vec<u8> {
        let mut k: Vec<u8> = Registration::get_prefix();
        k.append(&mut user_hash.to_ne_bytes().to_vec());
        k
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
    pub buttons: Vec<Vec<Vec<(String,String)>>>,
    pub user_hash: u64,
}
impl Notify {
    pub fn calculate_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.msg.hash(&mut s);
        self.buttons.hash(&mut s);
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
impl TryFrom<Vec<u8>> for UserQuery {
    type Error = anyhow::Error;
    fn try_from(item: Vec<u8>) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(&item[..])?)
    }
}
impl TryFrom<UserQuery> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(item: UserQuery) -> anyhow::Result<Self> {
        Ok(bincode::serialize(&item)?)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct SettingsPart {
    pub subscribe: Option<bool>,
    pub unsubscribe: Option<bool>,
    pub register: Option<bool>,
    pub user_hash: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum QueryPart {
    RegisterQueryPart(RegisterQueryPart),
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
            QueryPart::RegisterQueryPart(q) => {
                q.hash(state);
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EntriesQueryPart {
    pub message: String,
    pub display: String,
    pub indices: Vec<String>,
    pub filter: Vec<Vec<(String, String)>>,
    pub order_by: String,
    pub limit: usize,
}
impl Hash for EntriesQueryPart {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.message.hash(state);
        self.display.hash(state);
        self.indices.hash(state);
        /*
        let mut key_value_vector: Vec<String> = self.filter.iter().flatten().enumerate().map(|(i,x)| format!("{},{},{}",i,x.0,x.1)).collect();
        key_value_vector.sort_unstable();
        key_value_vector.join(";").hash(state);*/
        bincode::serialize(&self.filter).unwrap().hash(state);
        self.order_by.hash(state);
        self.limit.hash(state);
    }
}


#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct RegisterQueryPart {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct SubscriptionsQueryPart {
    pub message: String,
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

impl TryFrom<Vec<u8>> for CosmosRustServerValue {
    type Error = anyhow::Error;
    fn try_from(item: Vec<u8>) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(&item[..])?)
    }
}

impl TryFrom<CosmosRustServerValue> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(item: CosmosRustServerValue) -> anyhow::Result<Self> {
        Ok(bincode::serialize(&item)?)
    }
}

impl CosmosRustServerValue {
    pub fn key(&self) -> Vec<u8> {
        match self {
            CosmosRustServerValue::Notification(entry) => entry.get_key(),
            CosmosRustServerValue::Notify(entry) => entry.get_key(),
            CosmosRustServerValue::UserMetaData(entry) => entry.get_key(),
        }
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
    Registration(Registration),
}

impl TryFrom<Vec<u8>> for CosmosRustBotValue {
    type Error = anyhow::Error;
    fn try_from(item: Vec<u8>) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(&item[..])?)
    }
}

impl TryFrom<CosmosRustBotValue> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(item: CosmosRustBotValue) -> anyhow::Result<Self> {
        Ok(bincode::serialize(&item)?)
    }
}

impl CosmosRustBotValue {
    pub fn key(&self) -> Vec<u8> {
        match self {
            CosmosRustBotValue::Entry(entry) => entry.get_key(),
            CosmosRustBotValue::Index(index) => index.get_key(),
            CosmosRustBotValue::Subscription(sub) => sub.get_key(),
            CosmosRustBotValue::Registration(reg) => reg.get_key(),
        }
    }
    pub fn get(&self, field: &str) -> serde_json::Value {
        match self {
            CosmosRustBotValue::Entry(entry) => match entry {
                Entry::Value(val) => match field {
                    "timestamp" => serde_json::json!(val.timestamp),
                    "origin" => serde_json::json!(val.origin),
                    &_ => val.custom_data.get(field),
                },
            },
            CosmosRustBotValue::Index(val) => match field {
                "name" => serde_json::json!(val.name),
                "list" => serde_json::json!(val.list),
                &_ => serde_json::Value::Null,
            },
            CosmosRustBotValue::Subscription(val) => match field {
                "query" => serde_json::json!(val.query),
                "user_list" => serde_json::json!(val.user_list),
                "list" => serde_json::json!(val.list),
                &_ => serde_json::Value::Null,
            },
            CosmosRustBotValue::Registration(val) => match field {
                "token" => serde_json::json!(val.token),
                "user_hash" => serde_json::json!(val.user_hash),
                &_ => serde_json::Value::Null,
            },
        }
    }
    pub fn add_variants_of_memberships(view: &mut Vec<CosmosRustBotValue>, fields: Vec<&str>) {
        for field in fields {
            let variants = view
                .iter()
                .filter_map(|x| {
                    let val = x.get(field);
                    if val != serde_json::Value::Null{
                        return Some(val.as_str().unwrap().to_string());
                    }
                    return None;
                })
                .collect::<HashSet<String>>();
            for variant in variants {
                let entries = view
                    .iter()
                    .filter_map(|x| {
                        let val = x.get(field);
                        if val != serde_json::Value::Null{
                            if val.as_str().unwrap() == variant{
                                return Some(x.clone());
                            }
                        }
                        return None;
                    })
                    .collect::<Vec<CosmosRustBotValue>>();
                let membership = CosmosRustBotValue::create_membership(
                    &entries,
                    None,
                    format!("{}_{}", field, variant).as_str(),
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
            .filter_map(|x| {
                if let Some(f) = field {
                    if x.get(f) != serde_json::Value::Null {
                        return Some(x.key());
                    }
                }
                return None;
            })
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
            .filter_map(|x| {
                let val = x.get(field);
                if val != serde_json::Value::Null {
                    return Some((x.key(),val));
                }
                return None;
            })
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
