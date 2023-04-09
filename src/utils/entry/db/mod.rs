pub mod notification;
pub mod query;
pub mod socket;

use sled::{IVec, Mode};
use std::path::PathBuf;

use log::{debug, error, info, trace};

use crate::utils::entry::Entry;
use crate::utils::entry::Subscription;
use crate::utils::entry::CosmosRustBotValue;

use crate::utils::entry::CosmosRustServerValue;

use crate::utils::entry::*;

use crate::utils::entry::db::notification::socket::{client_send_notification_request};
use crate::utils::entry::db::query::socket::spawn_socket_query_server;

use std::collections::HashMap;
use crate::utils::response::ResponseResult;
use cosmos_rust_package::chrono::Utc;
use serde_json::json;
use cosmos_rust_package::tokio::time::{sleep, Duration};
use crate::utils::entry::ValueImperative::Notify;

use serde::{Serialize,Deserialize};
use crate::utils::entry::db::query::CosmosRustBotStoreInquirer;


const NOTIFICATION_SOCKET: &str = "./tmp/cosmos_rust_bot_notification_socket";

const REV_INDEX_PREFIX: &str = "rev_index_";

const CRB_SUBSCRIPTION_STORE_JSON: &str = "./tmp/cosmos_rust_bot_subscriptions.json";
const CRB_REGISTRATION_STORE_JSON: &str = "./tmp/cosmos_rust_bot_registrations.json";

pub fn load_sled_db(path: &str) -> sled::Db {
    let db: sled::Db = sled::Config::default()
        .path(path)
        .cache_capacity( 1024 * 1024 * 1024) // 1gb
        //.use_compression(true)
        //.compression_factor(22)
        .flush_every_ms(Some(1000))
        .mode(Mode::HighThroughput)
        .open()
        .unwrap();
    db
}


#[derive(Debug)]
pub enum RetrievalMethod {
    Get,
    GetOk,
}

pub struct TaskMemoryStore(SledStore);

impl Clone for TaskMemoryStore {
    fn clone(&self) -> Self {
        let sled_db_copy = self.0.db.clone();
        TaskMemoryStore(SledStore::new(sled_db_copy))
    }

    fn clone_from(&mut self, source: &Self) {
        let sled_db_copy = source.0.db.clone();
        *self = TaskMemoryStore(SledStore::new(sled_db_copy));
    }
}

impl TaskMemoryStore {
    pub fn new(path: Option<String>) -> anyhow::Result<Self> {
        let sled_store = if let Some(p) = path {
            SledStore::open(&p)?
        }else {
            SledStore::temporary()?
        };
        Ok(TaskMemoryStore(sled_store))
    }

    // Get: returns the max revision.
    // GetOk: returns the first ok result with max revision
    // the item stored and found with the given key must impl Deserialize for T, else an Error is returned.
    pub fn get<T>(&self, key: &str, retrieval_method: &RetrievalMethod) -> anyhow::Result<Maybe<T>>
        where
            T: for<'a> Deserialize<'a> + Serialize
    {

        let current_rev: Option<IVec> = self.0.get(format!("{}{}", REV_INDEX_PREFIX, key).as_bytes().to_vec())?;
        let index = match current_rev {
            Some(val) => u64::from_be_bytes(val.to_vec()[..].try_into()?),
            None => 0u64
        };

        let value = match retrieval_method {
            RetrievalMethod::Get => {
                let key = format!("key_{}_rev_{}",key,index);
                trace!("Get: {},", key);
                let item: Option<IVec> = self.0.get(key.as_bytes().to_vec())?;
                Ok(match item {
                    Some(val) => {
                        val.to_vec().try_into()?
                    },
                    None => Maybe {
                        data: Err(MaybeError::KeyDoesNotExist(key.to_string())),
                        timestamp: Utc::now().timestamp(),
                    },
                })
            }
            RetrievalMethod::GetOk => {
                for i in (0..=index).rev() {
                    let key = format!("key_{}_rev_{}",key,i);
                    trace!("GetOk: {}", key);
                    let item: Option<IVec> = self.0.get(key.as_bytes().to_vec())?;
                    match item {
                        Some(val) => {
                            let tmp: Maybe<T> = val.to_vec().try_into()?;
                            if let Maybe{ data: Ok(_),.. } = tmp {
                                return Ok(tmp);
                            }
                        },
                        None => {
                            break;
                        },
                    }
                }
                Err(anyhow::anyhow!("Error: no ok value found for key {}",key))
            }
        };
        trace!("{:?}: key: {:?}, value: {}", retrieval_method, key,match &value { Ok(v) => serde_json::to_string_pretty(v).unwrap_or("Formatting Error".to_string()), Err(e) => e.to_string()});
        value
    }

    pub fn get_index_of_ok_result<T>(&self, key: &str, index: u64) -> anyhow::Result<u64>
        where
            T: for<'a> Deserialize<'a> + Serialize
    {
        for i in (0..=index).rev() {
            let key = format!("key_{}_rev_{}", key, i);
            let item: Option<IVec> = self.0.get(key.as_bytes().to_vec())?;
            match item {
                Some(val) => {
                    let tmp: Maybe<T> = val.to_vec().try_into()?;
                    if let Maybe{ data: Ok(_),.. } = tmp {
                        return Ok(i);
                    }
                },
                None => {
                },
            }
        }
        Err(anyhow::anyhow!("Error: no index found for key {}",key))
    }

    pub fn contains_key(&self, key: &str) -> bool
    {
        let current_rev: Option<Option<IVec>> = self.0.get(format!("{}{}", REV_INDEX_PREFIX, key).as_bytes().to_vec()).ok();
        let res = match current_rev {
            Some(Some(val)) => true,
            Some(None) => false,
            None => false
        };
        trace!("contains_key: key: {}, value: {:?}", key, res);
        res
    }

    // removes all historic entries, starting from (exclusive) the last ok result
    pub fn remove_historic_entries<T>(&self, key: &str, max_index: u64) -> anyhow::Result<()>
        where
            T: for<'a> Deserialize<'a> + Serialize
    {
        trace!("remove_historic_entries: key: {}, max_index: {}", key, max_index);
        let smallest_required_index = self.get_index_of_ok_result::<T>(key, max_index).unwrap_or(max_index);
        for i in (0..smallest_required_index).rev() {
            if self.0.remove(format!("key_{}_rev_{}",&key,i).as_bytes().to_vec())?.is_none(){
                trace!("key does not exist: key: {}, index: {}", key, i);
                break;
            }else{
                trace!("removed: key: {}, index: {}", key, i);
            }
        }
        Ok(())
    }

    // increases revision and adds key/value pair to it.
    // uses `remove_historic_entries` to clean up the history.
    //
    // called in async/parallel from multiple threads
    pub fn push<T>(&self, key: &str, value: Maybe<T>) -> anyhow::Result<()>
        where
            T: for<'a> Deserialize<'a> + Serialize
    {
        trace!("push key: key: {}", key);
        trace!("push key: value: {}", serde_json::to_string_pretty(&value).unwrap_or("Formatting Error".to_string()));
        let current_rev: Option<IVec> = self.0.get(format!("{}{}", REV_INDEX_PREFIX, key).as_bytes().to_vec())?;
        let next_index = match current_rev {
            Some(val) => u64::from_be_bytes(val.to_vec()[..].try_into()?).overflowing_add(1),
            None => (0u64,false)
        };
        if next_index.1 { // in case of an overflow, the complete key history is wiped.
            trace!("push key: {}, overflow: {:?}", key, next_index);
            for i in (0..=u64::MAX).rev() {
                if self.0.remove(format!("key_{}_rev_{}",key,i).as_bytes().to_vec())?.is_none(){
                    break;
                }else{
                    trace!("removed: key: {}, index: {}", key, i);
                }
            }
        }
        let tmp: Vec<u8> = value.try_into()?;
        self.0.insert(format!("key_{}_rev_{}",key,next_index.0).as_bytes().to_vec(),tmp)?;
        self.0.insert(format!("{}{}",REV_INDEX_PREFIX,key).as_bytes().to_vec(),next_index.0.to_be_bytes().to_vec())?;

        self.remove_historic_entries::<T>(key, next_index.0)?;
        Ok(())
    }

    pub fn key_iter(&self) -> impl Iterator<Item = String> {
        let mut iter = self.0.db.scan_prefix(REV_INDEX_PREFIX.as_bytes());
        iter.filter_map(|x| {
            if let Ok((key,_)) = x {
                return match String::from_utf8(key.to_vec()) {
                    Ok(key) => Some(key[REV_INDEX_PREFIX.len()..].to_string()),
                    Err(_) => {None}
                };
            }
            return None
        })
    }

    pub fn value_iter<'b,T>(&'b self, retrieval_method: &'b RetrievalMethod) -> impl Iterator<Item = (String,Maybe<T>)> +'_
        where
            T: for<'a> Deserialize<'a> + Serialize
    {
        self.key_iter().map(|key| {
            match self.get::<T>(&key,retrieval_method) {
                Ok(val) => { (key, val) },
                Err(err) => {
                    let error = Err(MaybeError::AnyhowError(format!("Error: Key: {}, Err: {}",&key, err.to_string())));
                    (key,
                     Maybe {
                         data: error,
                         timestamp: Utc::now().timestamp(),
                     })
                },
            }
        })
    }
}


pub struct CosmosRustBotStore {
    pub entry_store: EntryStore,
    pub index_store: IndexStore,
    pub subscription_store: SubscriptionStore,
}

impl Clone for CosmosRustBotStore {
    fn clone(&self) -> Self {
        CosmosRustBotStore {
            entry_store: EntryStore::new(&self.entry_store.0.db),
            index_store: IndexStore::new(&self.index_store.0.db),
            subscription_store: SubscriptionStore::new(&self.subscription_store.0.db),
        }
    }
}

impl CosmosRustBotStore {

    pub fn new(entry_index_db: sled::Db, subscription_store: SubscriptionStore) -> Self {
        CosmosRustBotStore {
            entry_store: EntryStore::new(&entry_index_db),
            index_store: IndexStore::new(&entry_index_db),
            subscription_store,
        }
    }

    pub fn update_items(&mut self, mut items: Vec<CosmosRustBotValue>) {

        self.entry_store.remove_entries_not_in_items(&items); // outdated entries/indices
        self.index_store.remove_indices_not_in_items(&items);

        for item in &items {

            match item {  // insert updated entries/indices (hash/key changed)
                CosmosRustBotValue::Entry(entry) => {
                    let key =item.key();
                    let value: Vec<u8> = item.clone().try_into().unwrap();
                    if let Ok(false) = self.entry_store.0.db.contains_key(&key) {
                        self.entry_store.0.db.insert(&key, value).ok();

                        if let Entry::Value(Value { timestamp: _, origin: _, custom_data: CustomData::ProposalData(proposal_data), imperative: _ }) = entry.clone() {
                            // filesystem sync of generated files for proposal data
                            let file_path = format!("./tmp/public/en/{}/{}.json", proposal_data.proposal_blockchain.to_lowercase(), proposal_data.proposal_id);
                            match std::fs::write(&file_path, serde_json::to_string_pretty(&proposal_data.generate_map()).unwrap()) {
                                Ok(_) => {},
                                Err(err) => { error!("Unable to write {}, Error: {}", &file_path,err.to_string()); },
                            };

                            // write governance proposal as HTML page
                            let file_path = format!("./tmp/governance_proposals/{}/{}.html", proposal_data.proposal_blockchain.to_lowercase(), proposal_data.proposal_id);
                            match std::fs::write(&file_path, proposal_data.generate_html()) {
                                Ok(_) => {},
                                Err(err) => { error!("Unable to write {}, Error: {}", &file_path,err.to_string()); },
                            };
                            // write the fraud prediction to a JSON
                            let file_path = format!("./tmp/fraud_detection/{}/{}.json", proposal_data.proposal_blockchain.to_lowercase(), proposal_data.proposal_id);
                            let json_string = json!({"title": proposal_data.proposal_title, "description": proposal_data.proposal_description, "fraud_prediction": proposal_data.fraud_risk}).to_string();
                            match std::fs::write(&file_path, json_string) {
                                Ok(_) => {},
                                Err(err) => { error!("Unable to write {}, Error: {}", &file_path,err.to_string()); },
                            };
                        }
                    }
                },
                CosmosRustBotValue::Index(_) => {
                    let key =item.key();
                    let value: Vec<u8> = item.clone().try_into().unwrap();
                    if let Ok(false) = self.index_store.0.db.contains_key(&key) {
                        self.index_store.0.db.insert(&key, value).ok();
                    }
                }
                _ => {}
            };
        }

        self.update_outdated_subscriptions();

    }

    fn update_outdated_subscriptions(&mut self) {
        // refreshing subscriptions by updating them if their content changed.
        for mut subscription in self.subscription_store.get_subscriptions() {
            if self.subscription_outdated(&mut subscription){
                subscription.action = SubscriptionAction::Update;
                let item = CosmosRustBotValue::Subscription(subscription);
                let key = item.key();
                let value: Vec<u8> = item.try_into().unwrap();
                self.subscription_store.0.db.insert(&key, value).ok();
            }
        }
    }

    fn subscription_outdated(&mut self, subscription: &mut Subscription) -> bool {

        if let QueryPart::EntriesQueryPart(query_part) = &subscription.query
        {
            let query_result = CosmosRustBotStoreInquirer(&self).entries_query(query_part);

            let mut added_items = false;
            let mut removed_items = false;

            let filtered_keys =
                query_result.iter().map(|x| x.key()).collect::<Vec<Vec<u8>>>();
            for e in &filtered_keys {
                if !subscription.list.contains(&e) {
                    added_items = true;
                    subscription.list.push(e.clone());
                }
            }
            let len = subscription.list.len();
            subscription.list.retain(|x| filtered_keys.contains(x));
            if len != subscription.list.len() {
                removed_items = true;
            }
            return added_items || removed_items;
        }
        false
    }

    pub fn spawn_notify_on_subscription_update_task(&mut self) -> cosmos_rust_package::tokio::task::JoinHandle<()> {
        let mut copy_self = self.clone();
        cosmos_rust_package::tokio::spawn(async move {

            // Delay the task for 5 minute (5*60 seconds)
            // Reason: This delay allows the state to get up-to-date first.
            sleep(Duration::from_secs(5*60)).await;

            copy_self.subscription_store.register_subscriber().unwrap();

            while let Some(updated) = copy_self.subscription_store.get_next_updated() {

                if let Ok(CosmosRustBotValue::Subscription(s)) = updated {
                    if s.action == SubscriptionAction::Update  {

                        let query: UserQuery = UserQuery::new(s.query);

                        let mut entries = CosmosRustBotStoreInquirer(&copy_self).query(&query);
                        entries.retain(|x| {
                            if let CosmosRustBotValue::Entry(Entry::Value(v)) = x {
                                v.imperative == ValueImperative::Notify
                            } else {
                                true
                            }
                        });
                        if !entries.is_empty() {
                            let notification = Notification {
                                query,
                                entries,
                                user_list: s.user_list,
                            };
                            client_send_notification_request(
                                NOTIFICATION_SOCKET,
                                CosmosRustServerValue::Notification(notification),
                            ).ok();
                        }
                    }
                }
            }
        })
    }
}


pub struct IndexStore(SledStore);

impl IndexStore {

    pub fn new(tree: &sled::Db) -> Self {
        let sled_store = SledStore::new(tree.clone());
        IndexStore(sled_store)
    }

    pub fn get_indices(&self) -> impl Iterator<Item = CosmosRustBotValue> {
        self.0.db.scan_prefix(Index::get_prefix()).filter_map(|item| match item {
            Ok((_k, v)) => {
                let maybe_index = v.to_vec().try_into().unwrap();
                match maybe_index {
                    CosmosRustBotValue::Index(_) => {
                        Some(maybe_index)
                    },
                    _ => {None}
                }
            },
            Err(_e) => {
                None
            }
        })
    }

    pub fn remove_indices_not_in_items(&mut self, items: &Vec<CosmosRustBotValue>) {

        let item_keys = items
            .iter()
            .map(|x| x.key())
            .collect::<Vec<Vec<u8>>>();

        for index in self.get_indices() {
            let key = index.key();
            if !item_keys.contains(&key) {
                self.0.db.remove(key).ok();
            }
        }
    }

    pub fn register_subscriber(&mut self) -> anyhow::Result<()> {
        self.0.set_subscriber()?;
        self.0.subscriber = Some(self.0.db.watch_prefix(Index::get_prefix()));
        Ok(())
    }
}

pub struct EntryStore(SledStore);

impl EntryStore {

    pub fn new(tree: &sled::Db) -> Self {
        let sled_store = SledStore::new(tree.clone());
        EntryStore(sled_store)
    }

    pub fn get_entries(&self) -> impl Iterator<Item = CosmosRustBotValue> {
        self.0.db.scan_prefix(Entry::get_prefix()).filter_map(|item| match item {
            Ok((_k, v)) => {
                let maybe_entry = v.to_vec().try_into().unwrap();
                match maybe_entry {
                    CosmosRustBotValue::Entry(_) => {
                        Some(maybe_entry)
                    },
                    _ => {None}
                }
            },
            Err(_e) => {
                None
            }
        })
    }

    pub fn remove_entries_not_in_items(&mut self, items: &Vec<CosmosRustBotValue>) {

        let item_keys = items
            .iter()
            .map(|x| x.key())
            .collect::<Vec<Vec<u8>>>();

        for entry in self.get_entries() {
            let key = entry.key();
            if !item_keys.contains(&key) {
                self.0.db.remove(key).ok();
            }
        }
    }

    pub fn register_subscriber(&mut self) -> anyhow::Result<()> {
        self.0.set_subscriber()?;
        self.0.subscriber = Some(self.0.db.watch_prefix(Entry::get_prefix()));
        Ok(())
    }
}

pub struct SubscriptionStore(SledStore);

impl SubscriptionStore {

    pub fn new(tree: &sled::Db) -> Self {
        let sled_store = SledStore::new(tree.clone());
        SubscriptionStore(sled_store)
    }

    pub fn get_subscriptions(&self) -> impl Iterator<Item = Subscription> {
        self.0.db.scan_prefix(Subscription::get_prefix()).filter_map(|item| match item {
            Ok((_k, v)) => {
                match v.to_vec().try_into().unwrap() {
                    CosmosRustBotValue::Subscription(sub) => {
                        Some(sub)
                    },
                    _ => {None}
                }
            },
            Err(_e) => {
                None
            }
        })
    }

    pub fn get_registrations(&self) -> impl Iterator<Item = Registration> {
        self.0.db.scan_prefix(Registration::get_prefix()).filter_map(|item| match item {
            Ok((_k, v)) => {
                match v.to_vec().try_into().unwrap() {
                    CosmosRustBotValue::Registration(reg) => {
                        Some(reg)
                    },
                    _ => {None}
                }
            },
            Err(_e) => {
                None
            }
        })
    }

    pub fn export_subscriptions(&self, path: &str){
        let json = serde_json::json!(self.get_subscriptions().collect::<Vec<Subscription>>());
        if let Ok(serialized) = serde_json::to_string_pretty(&json) {
            std::fs::write(path, serialized).ok();
        }
    }

    pub fn export_registrations(&self, path: &str){
        let json = serde_json::json!(self.get_registrations().collect::<Vec<Registration>>());
        if let Ok(serialized) = serde_json::to_string_pretty(&json) {
            std::fs::write(path, serialized).ok();
        }
    }

    pub fn import_subscriptions(&self, path: &str){
        if let Ok(contents) = std::fs::read_to_string(path){
            if let Ok(subscriptions) = serde_json::from_str::<Vec<Subscription>>(&contents){
                for subscription in subscriptions {
                    let item = CosmosRustBotValue::Subscription(subscription);
                    let key = item.key();
                    let value: Vec<u8> = item.try_into().unwrap();
                    self.0.db.insert(&key, value).ok();
                }
            }
        }
    }

    pub fn import_registrations(&self, path: &str){
        if let Ok(contents) = std::fs::read_to_string(path){
            if let Ok(registrations) = serde_json::from_str::<Vec<Registration>>(&contents){
                for registration in registrations {
                    let item = CosmosRustBotValue::Registration(registration);
                    let key = item.key();
                    let value: Vec<u8> = item.try_into().unwrap();
                    self.0.db.insert(&key, value).ok();
                }
            }
        }
    }

    pub fn get_next_updated(&mut self) -> Option<anyhow::Result<CosmosRustBotValue>> {
        match self.0.await_next_update() {
            Some(sled::Event::Remove { key }) => {
                Some(Err(anyhow::anyhow!("Error: Remove Event.")))
            },
            Some(sled::Event::Insert { key, value }) => {
                match value.to_vec().try_into().unwrap() {
                    CosmosRustBotValue::Subscription(s) => {
                        self.export_subscriptions(CRB_SUBSCRIPTION_STORE_JSON);
                        Some(Ok(CosmosRustBotValue::Subscription(s)))
                    },
                    CosmosRustBotValue::Registration(r) => {
                        self.export_subscriptions(CRB_REGISTRATION_STORE_JSON);
                        Some(Ok(CosmosRustBotValue::Registration(r)))
                    },
                    _ => {
                        Some(Err(anyhow::anyhow!("Error: Unexpected Type.")))
                    }
                }
            },
            _ => { None }
        }
    }

    pub fn register_subscriber(&mut self) -> anyhow::Result<()> {
        self.0.set_subscriber()?;
        self.0.subscriber = Some(self.0.db.watch_prefix(Subscription::get_prefix()));
        Ok(())
    }
}

pub struct SledStore {
    db: sled::Db,
    subscriber: Option<sled::Subscriber>,
}

impl SledStore {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let db: sled::Db = load_sled_db(path);
        Ok(SledStore::new(db))
    }
    pub fn new(sled_db: sled::Db) -> Self {
        SledStore {
            db: sled_db,
            subscriber: None,
        }
    }

    pub fn temporary() -> anyhow::Result<Self> {
        Ok(Self {
            db: sled::Config::new().temporary(true).cache_capacity(1024 * 1024 * 1024 * 2 /*2GB*/).open()?,
            subscriber: None,
        })
    }


    pub fn get_tree(&self) -> &sled::Db {
        &self.db
    }



    fn contains_key<K>(&self, key: K) -> anyhow::Result<bool>
        where
            K: AsRef<Vec<u8>>,
    {
        trace!("contains_key {:?}", key.as_ref());
        Ok(self.db.contains_key(key.as_ref())?)
    }

    fn get<K>(&self, key: K) -> sled::Result<Option<IVec>>
        where
            K: AsRef<Vec<u8>>,
    {
        trace!("get {:?}", key.as_ref());
        Ok(self.db.get(key.as_ref())?)
    }

    fn insert<K, V>(&self, key: K, value: V) -> anyhow::Result<()>
        where
            K: AsRef<Vec<u8>>,
            IVec: From<V>,
    {
        trace!("inserting {:?}", key.as_ref());
        let _ = self
            .db
            .insert(key.as_ref(), value)?;
        Ok(())
    }

    fn remove<S>(&self, key: S) -> anyhow::Result<Option<sled::IVec>>
        where
            S: AsRef<Vec<u8>>,
    {
        trace!("removing {:?} from db", key.as_ref());
        Ok(self.db.remove(key.as_ref())?)
    }

    fn set_subscriber(&self)  -> anyhow::Result<()> {
        if self.subscriber.is_some() {
            Err(anyhow::anyhow!("Error: Subscriber already exists."))
        }else {
            Ok(())
        }
    }

    fn register_subscriber<S>(&mut self,  prefix: S) -> anyhow::Result<()>
        where
            S: AsRef<Vec<u8>> + std::convert::AsRef<[u8]>,
    {
        self.set_subscriber()?;
        self.subscriber = Some(self.db.watch_prefix(prefix));
        Ok(())
    }

    fn await_next_update(&mut self) -> Option<sled::Event> {
        self.subscriber.as_mut().map(|s| s.next()).flatten()
    }
}
