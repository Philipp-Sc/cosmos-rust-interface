pub mod notification;
pub mod query;
pub mod socket;

use sled::IVec;
use std::path::PathBuf;

use log::{debug, info, trace};

use crate::utils::entry::Entry;
use crate::utils::entry::Subscription;
use crate::utils::entry::CosmosRustBotValue;

use crate::utils::entry::CosmosRustServerValue;

use crate::utils::entry::*;

use crate::utils::entry::db::notification::socket::{client_send_notification_request};
use crate::utils::entry::db::query::{handle_query_sled_db, query_entries_sled_db, update_subscription};
use crate::utils::entry::db::query::socket::spawn_socket_query_server;

use std::collections::HashMap;
use crate::utils::response::ResponseResult;
use chrono::Utc;


const NOTIFICATION_SOCKET: &str = "./tmp/cosmos_rust_bot_notification_socket";

const REV_INDEX_PREFIX: &str = "rev_index_";

pub fn load_sled_db(path: &str) -> sled::Db {
    let db: sled::Db = sled::Config::default()
        .path(path.to_owned())
        .cache_capacity(1024 * 1024 * 1024 / 2)
        .use_compression(true)
        .compression_factor(22)
        .flush_every_ms(Some(1000))
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
    pub fn new() -> anyhow::Result<Self> {
        let sled_store = SledStore::temporary()?;
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
                debug!("Get: {},", key);
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
                    debug!("GetOk: {}", key);
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
        debug!("{:?}: key: {:?}, value: {}", retrieval_method, key,match &value { Ok(v) => serde_json::to_string_pretty(v).unwrap_or("Formatting Error".to_string()), Err(e) => e.to_string()});
        value
    }

    pub fn get_index_of_ok_result(&self, key: &str, index: u64) -> anyhow::Result<u64>
    {
        for i in (0..=index).rev() {
            let key = format!("key_{}_rev_{}", key, i);
            let item: Option<IVec> = self.0.get(key.as_bytes().to_vec())?;
            if item.is_some(){
                return Ok(i);
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
        info!("contains_key: key: {}, value: {:?}", key, res);
        res
    }

    // removes all historic entries, starting from (exclusive) the last ok result
    pub fn remove_historic_entries(&self, key: &str, index: u64) -> anyhow::Result<()> {

        info!("remove_historic_entries: key: {}, index: {}", key, index);
        let smallest_required_index = self.get_index_of_ok_result(key, index).unwrap_or(index);
        for i in (0..smallest_required_index).rev() {
            if self.0.remove(format!("key_{}_rev_{}",&key,i).as_bytes().to_vec())?.is_none(){
                info!("key does not exist: key: {}, index: {}", key, i);
                break;
            }else{
                info!("removed: key: {}, index: {}", key, i);
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
            T: Serialize
    {
        info!("push key: key: {}", key);
        debug!("push key: value: {}", serde_json::to_string_pretty(&value).unwrap_or("Formatting Error".to_string()));
        let current_rev: Option<IVec> = self.0.get(format!("{}{}", REV_INDEX_PREFIX, key).as_bytes().to_vec())?;
        let next_index = match current_rev {
            Some(val) => u64::from_be_bytes(val.to_vec()[..].try_into()?).overflowing_add(1),
            None => (0u64,false)
        };
        if next_index.1 { // in case of an overflow, the complete key history is wiped.
            info!("push key: {}, overflow: {:?}", key, next_index);
            for i in (0..=u64::MAX).rev() {
                if self.0.remove(format!("key_{}_rev_{}",key,i).as_bytes().to_vec())?.is_none(){
                    break;
                }else{
                    info!("removed: key: {}, index: {}", key, i);
                }
            }
        }
        self.0.insert(format!("{}{}",REV_INDEX_PREFIX,key).as_bytes().to_vec(),next_index.0.to_be_bytes().to_vec())?;
        let tmp: Vec<u8> = value.try_into()?;
        self.0.insert(format!("key_{}_rev_{}",key,next_index.0).as_bytes().to_vec(),tmp)?;

        self.remove_historic_entries(key, next_index.0)?;
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
            return match self.get::<T>(&key,retrieval_method) {
                Ok(val) => { (key, val) },
                Err(err) => {
                    let error = Err(MaybeError::AnyhowError(format!("Error: Key: {}, Err: {}",&key, err.to_string())));
                    (key,
                     Maybe {
                         data: error,
                         timestamp: Utc::now().timestamp(),
                     })
                },
            };
        })
    }
}


pub struct CosmosRustBotStore {
    pub entry_store: EntryStore,
    pub index_store: IndexStore,
    pub subscription_store: SubscriptionStore,
    pub sled_store: SledStore,
}

impl CosmosRustBotStore {

    pub fn new(tree: &sled::Db) -> Self {
        CosmosRustBotStore {
            entry_store: EntryStore::new(tree),
            index_store: IndexStore::new(tree),
            subscription_store: SubscriptionStore::new(tree),
            sled_store: SledStore::new(tree.clone()),
        }
    }

    pub fn spawn_thread_notify_on_subscription_update(&mut self) -> tokio::task::JoinHandle<()> {
        self.subscription_store.notify_on_subscription_update()
    }

    pub fn update_items(&mut self, mut items: Vec<CosmosRustBotValue>) {
        let mut batch = sled::Batch::default();

        self.entry_store.remove_outdated_entries(&items,&mut batch);
        self.index_store.remove_outdated_indices(&items,&mut batch);

        self.entry_store.retain_items_not_in_entry_store(&mut items);
        self.index_store.retain_items_not_in_index_store(&mut items);

        for x in 0..items.len() {
            batch.insert(sled::IVec::from(items[x].key()), items[x].value());
        }
        self.subscription_store.update_subscriptions_if_content_modified(&items,&mut batch);

        self.sled_store.get_tree().apply_batch(batch).unwrap();
    }
}


pub struct IndexStore(SledStore);

impl IndexStore {

    pub fn new(tree: &sled::Db) -> Self {
        let sled_store = SledStore::new(tree.clone());
        IndexStore(sled_store)
    }

    pub fn get_indices(&self) -> Vec<Index> {
        self.0.db.scan_prefix(Index::get_prefix()).filter_map(|item| match item {
            Ok((_k, v)) => {
                match CosmosRustBotValue::from(v.to_vec()) {
                    CosmosRustBotValue::Index(index) => {
                        Some(index)
                    },
                    _ => {None}
                }
            },
            Err(_e) => {
                None
            }
        }).collect::<Vec<Index>>()
    }

    pub fn remove_outdated_indices(&mut self, items: &Vec<CosmosRustBotValue>, batch: &mut sled::Batch) {

        let item_keys = items
            .iter()
            .map(|x| x.key())
            .collect::<Vec<Vec<u8>>>();

        for index in self.get_indices() {
            let key = CosmosRustBotValue::Index(index).key();
            if !item_keys.contains(&key) {
                batch.remove(key);
            }
        }
    }

    pub fn retain_items_not_in_index_store(&self, items: &mut Vec<CosmosRustBotValue>) {
        let index_keys = self.get_indices()
            .into_iter()
            .map(|x| CosmosRustBotValue::Index(x).key())
            .collect::<Vec<Vec<u8>>>();

        items.retain(|x| !index_keys.contains(&x.key()));
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

    pub fn get_entries(&self) -> Vec<Entry> {
        self.0.db.scan_prefix(Entry::get_prefix()).filter_map(|item| match item {
            Ok((_k, v)) => {
                match CosmosRustBotValue::from(v.to_vec()) {
                    CosmosRustBotValue::Entry(entry) => {
                        Some(entry)
                    },
                    _ => {None}
                }
            },
            Err(_e) => {
                None
            }
        }).collect::<Vec<Entry>>()
    }

    pub fn remove_outdated_entries(&mut self, items: &Vec<CosmosRustBotValue>, batch: &mut sled::Batch) {

        let item_keys = items
            .iter()
            .map(|x| x.key())
            .collect::<Vec<Vec<u8>>>();

        for entry in self.get_entries() {
            let key = CosmosRustBotValue::Entry(entry).key();
            if !item_keys.contains(&key) {
                batch.remove(key);
            }
        }
    }

    pub fn retain_items_not_in_entry_store(&self, items: &mut Vec<CosmosRustBotValue>) {
        let index_keys = self.get_entries()
            .into_iter()
            .map(|x| CosmosRustBotValue::Entry(x).key())
            .collect::<Vec<Vec<u8>>>();

        items.retain(|x| !index_keys.contains(&x.key()));
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

    pub fn get_subscriptions(&self) -> Vec<Subscription> {
        self.0.db.scan_prefix(Subscription::get_prefix()).filter_map(|item| match item {
            Ok((_k, v)) => {
                match CosmosRustBotValue::from(v.to_vec()) {
                    CosmosRustBotValue::Subscription(sub) => {
                        Some(sub)
                    },
                    _ => {None}
                }
            },
            Err(_e) => {
                None
            }
        }).collect::<Vec<Subscription>>()
    }

    pub fn get_next_updated_subscription(&mut self) -> Option<anyhow::Result<Subscription>> {
        match self.0.await_next_update() {
            Some(sled::Event::Remove { key }) => {
                Some(Err(anyhow::anyhow!("Error: Remove Event.")))
            },
            Some(sled::Event::Insert { key, value }) => {
                match CosmosRustBotValue::from(value.to_vec()){
                    CosmosRustBotValue::Subscription(s) => {
                        Some(Ok(s))
                    },
                    _ => {
                        Some(Err(anyhow::anyhow!("Error: Unexpected Type.")))
                    }
                }
            },
            _ => { None }
        }
    }
    pub fn update_subscriptions_if_content_modified(&self, entries: &Vec<CosmosRustBotValue>, batch: &mut sled::Batch) {
        // refreshing subscriptions by updating them if their content changed.
        for mut subscription in self.get_subscriptions() {
            if let Ok(_) = update_subscription(entries, &mut subscription){
                let item = CosmosRustBotValue::Subscription(subscription);
                batch.insert(item.key(),item.value());
            }
        }
    }

    pub fn notify_on_subscription_update(&mut self) -> tokio::task::JoinHandle<()> {
        let mut copy_self = SubscriptionStore::new(&self.0.db.clone());
        copy_self.register_subscriber();
        tokio::spawn(async move {
            while let Some(updated) = copy_self.get_next_updated_subscription() {
                if let Ok(s) = updated {
                    let query: UserQuery = UserQuery {
                        query_part: s.query,
                        settings_part: SettingsPart {
                            subscribe: None,
                            unsubscribe: None,
                            user_hash: None,
                        }
                    };
                    let entries = handle_query_sled_db(&copy_self.0.db, &query);
                    let notification = Notification {
                        query,
                        entries,
                        user_list: s.user_list,
                    };
                    // notify
                    client_send_notification_request(
                        NOTIFICATION_SOCKET,
                        CosmosRustServerValue::Notification(notification),
                    ).ok();
                }
            }
        })
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
    pub fn open(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let db: sled::Db = sled::Config::default()
            .path(path.into())
            .cache_capacity(1024 * 1024 * 1024 / 2)
            .use_compression(true)
            .compression_factor(22)
            .flush_every_ms(Some(1000))
            .open()?;
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
            db: sled::Config::new().temporary(true).open()?,
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
