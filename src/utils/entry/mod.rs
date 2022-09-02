
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
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
            Maybe { data: Err(err), timestamp } => Maybe { data: Err(anyhow::anyhow!(err.to_string())), timestamp: *timestamp },
            Maybe { data: Ok(value), timestamp } => Maybe { data: Ok(value.clone()), timestamp: *timestamp },
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
    pub fn new(timestamp: i64, origin: String, summary: String, custom_data: serde_json::Value) -> Value {
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
        self.timestamp.hash(state);
        self.origin.hash(state);
        self.summary.hash(state);
        self.custom_data.to_string().hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct Debug {
    pub timestamp: i64,
    pub origin: String,
    pub key: String,
    pub value: String,
    //..
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct Error {
    pub timestamp: i64,
    pub origin: String,
    pub key: String,
    pub value: String,
    pub summary: String,
    pub kind: String,
    //..
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub struct Log {
    pub timestamp: i64,
    pub origin: String,
    pub key: String,
    pub value: String,
    pub summary: String,
    //..
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
pub enum Entry {
    MetaData(MetaData),
    Debug(Debug),
    Error(Error),
    Log(Log),
    Value(Value)
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
pub struct Index { // may contain members or an ordering
    pub name: String,
    pub list: Vec<Vec<u8>>
}
impl Index {
    fn calculate_hash(name: &str) -> u64 {
        let mut s = DefaultHasher::new();
        name.hash(&mut s);
        s.finish()
    }
    fn get_hash(&self) -> u64 {
        Index::calculate_hash(self.name.as_str())
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
}

impl CosmosRustBotValue {
    pub fn key(&self) -> Vec<u8> {
        let mut k: Vec<u8> = Vec::new();
        match self {
            CosmosRustBotValue::Entry(entry) => {
                entry.get_key()
            },
            CosmosRustBotValue::Index(index) => {
                index.get_key()
            }
        }
    }
    pub fn value(&self) -> Vec<u8>{
        bincode::serialize(&self).unwrap()
    }
    pub fn from(value: Vec<u8>) -> CosmosRustBotValue {
        bincode::deserialize(&value[..]).unwrap()
    }
    pub fn try_get(&self,field: &str) -> Option<serde_json::Value> {
        match self {
            CosmosRustBotValue::Entry(entry) => {
                match entry {
                    Entry::Value(val) => {
                        match field {
                            "timestamp" => {
                                Some(serde_json::json!(val.timestamp))
                            },
                            "origin" => {
                                Some(serde_json::json!(val.origin))
                            },
                            "summary" => {
                                Some(serde_json::json!(val.summary))
                            },
                            &_ => {
                                val.custom_data().get(field).map(|x| x.clone())
                            }
                        }
                    },
                    Entry::MetaData(val) => {
                        match field {
                            "index" => {
                                Some(serde_json::json!(val.index))
                            },
                            "timestamp" => {
                                Some(serde_json::json!(val.timestamp))
                            },
                            "origin" => {
                                Some(serde_json::json!(val.origin))
                            },
                            "kind" => {
                                Some(serde_json::json!(val.kind))
                            },
                            "state" => {
                                Some(serde_json::json!(val.state))
                            },
                            "value" => {
                                Some(serde_json::json!(val.value))
                            },
                            "summary" => {
                                Some(serde_json::json!(val.summary))
                            },
                            &_ => {
                                None
                            }
                        }
                    },
                    Entry::Debug(_) | Entry::Error(_) | Entry::Log(_) => {None}
                }
            },
            CosmosRustBotValue::Index(val) => {
                match field {
                    "name" => {
                        Some(serde_json::json!(val.name))
                    },
                    "list" => {
                        Some(serde_json::json!(val.list))
                    },
                    &_ => {
                       None
                    }
                }
            }
        }
    }
    pub fn add_variants_of_memberships(view: &mut Vec<CosmosRustBotValue>, fields: Vec<&str>){
        for field in fields {
            let variants = view.iter().filter(|x| x.try_get(field).is_some()).map(|x| {
                x.try_get(field).unwrap().as_str().unwrap().to_string()
            }).collect::<HashSet<String>>();
            for variant in variants {
                let entries = view.iter().filter(|x| {
                    match x.try_get(field){
                        Some(t) => {t.as_str().unwrap() == variant},
                        None => {false}
                    }}).map(|x| x.clone()).collect::<Vec<CosmosRustBotValue>>();
                let membership = CosmosRustBotValue::create_membership(&entries, None, format!("{}_{}",field,variant.to_lowercase()).as_str());
                view.push(CosmosRustBotValue::Index(membership));
            }
        }
    }
    pub fn add_membership(entries: &mut Vec<CosmosRustBotValue>, field: Option<&str>, name: &str) {
        let index = CosmosRustBotValue::create_membership(entries,field,name);
        entries.push(CosmosRustBotValue::Index(index));
    }
    pub fn create_membership(entries: &Vec<CosmosRustBotValue>,field: Option<&str>, name: &str) -> Index {
        let have_field = entries.iter().map(|x| {
            if let Some(f) = field {
                (x.key(), x.try_get(f))
            }else{
                (x.key(), Some(serde_json::Value::Null))
            }
            }).filter(|(_,x)| {x.is_some()}).map(|(key,_)| key).collect::<Vec<Vec<u8>>>();
        Index {
            name: name.to_string(),
            list: have_field,
        }
    }
    pub fn add_index(entries: &mut Vec<CosmosRustBotValue>, field: &str, name: &str) {
        let index = CosmosRustBotValue::create_index(entries,field,name);
        entries.push(CosmosRustBotValue::Index(index));
    }
    pub fn create_index(entries: &Vec<CosmosRustBotValue>,field: &str, name: &str) -> Index {
        let mut have_field = entries.iter().map(|x| {(x.key(),x.try_get(field))}).filter(|(_,x)| {x.is_some()}).map(|(key,x)| (key,x.unwrap())).collect::<Vec<(Vec<u8>,serde_json::Value)>>();
        have_field.sort_by(|(_,first), (_, second)| {
            match (first,second) {
                (serde_json::Value::String(f),serde_json::Value::String(s)) => {
                    f.cmp(s)
                },
                (serde_json::Value::Number(f),serde_json::Value::Number(s)) => {
                    if f.is_u64() && s.is_u64() {
                        f.as_u64().unwrap().cmp(&s.as_u64().unwrap())
                    }else if f.is_i64() && s.is_i64() {
                        f.as_i64().unwrap().cmp(&s.as_i64().unwrap())
                    }else if f.is_f64() && s.is_f64() {
                        f.as_f64().unwrap().total_cmp(&s.as_f64().unwrap())
                    }else{
                        Ordering::Equal
                    }
                },
                _ => {
                    first.to_string().cmp(&second.to_string())
                }
            }
        });
        Index {
            name: name.to_string(),
            list: have_field.into_iter().rev().map(|(key, _)| {key}).collect(),
        }
    }
}

// add a function that creates Index for each origin given Vec<CosmosRustBotValue> and appends it to the vector.


/*


impl Entry {
    pub fn new(timestamp: i64, key: &str, value: EntryValue) -> Entry {
        Entry {
            timestamp,
            origin: key.to_string(),
            value,
        }
    }
}*/