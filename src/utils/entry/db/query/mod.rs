pub mod socket;

use crate::utils::entry::db::CosmosRustBotStore;
use crate::utils::entry::*;
use rand::{Rng, thread_rng};


pub struct CosmosRustBotStoreInquirer<'a>(pub &'a CosmosRustBotStore);


impl <'a>CosmosRustBotStoreInquirer<'a> {

    pub fn query(&mut self, query: &UserQuery) -> Vec<CosmosRustBotValue> {

        match &query.query_part {
            QueryPart::EntriesQueryPart(query_part) => {
                let result = self.entries_query(query_part);
                self.subscribe_unsubscribe_for_user(&result,query_part,&query.settings_part);
                result
            },
            QueryPart::SubscriptionsQueryPart(query_part) => {
                self.opt_unsubscribe_and_get_subscriptions_for_user(query_part, &query.settings_part)
            }
            QueryPart::RegisterQueryPart(_query_part) => {
                self.register_and_get_token_for_user(&query.settings_part)
            }
            QueryPart::AuthQueryPart(query_part) => {
                self.verify_auth_token(query_part)
            }
            /*
            QueryPart::RequestTranslationQueryPart(query_part) => {

            }
            requests if the user is allowed / limits quota (user management)
            */
        }
    }

    pub fn entries_query(&self, query_part: &EntriesQueryPart) -> Vec<CosmosRustBotValue> {

        // Clone the filter in query_part to avoid any modifications to the original filter
        let mut filter = query_part.filter.clone();

        // Get the order_by and limit from the query_part
        let order_by: Option<&str> = Some(&query_part.order_by);
        let limit: Option<usize> = Some(query_part.limit);

        // Create a new vector of vector of vector of bytes to hold the indices list
        let mut indices_list: Vec<Vec<Vec<u8>>> = Vec::new();

        // Initialize the order_by_index to None
        let mut order_by_index: Option<Vec<Vec<u8>>> = None;

        // Get all the indices from the index store and check if the index applies to the query.
        // If it does, add the index's list to the indices_list and remove the unnecessary filters for the index.
        for index in self.0.index_store.get_indices().filter_map(|x| if let CosmosRustBotValue::Index(index) = x { Some(index)}else{None}) {

            let index_applies = query_part.indices.contains(&index.name);
            if index_applies {
                indices_list.push(index.list.clone());

                for i in 0..filter.len() {
                    let filter_unnecessary = filter[i].iter().filter(|(k, v)| format!("{}_{}", k, v) == index.name).count() > 0;

                    if filter_unnecessary {
                        filter[i].retain(|(k, v)| format!("{}_{}", k, v) != index.name);
                    }
                }
            }

            // Check if order_by is present and set order_by_index to the list of the index with the same name as order_by
            if let Some(ord) = order_by {
                if &index.name == &ord {
                    order_by_index = Some(index.list.clone());
                }
            }
        }

        // Inner join the indices_list
        inner_join_vec(&mut indices_list);

        // Initialize selection to an empty vector
        let mut selection: Vec<Vec<u8>> = Vec::new();

        // Add the first vector in indices_list to the selection if indices_list is not empty
        if indices_list.len()>0 {
            selection.append(&mut indices_list[0]);
        }

        // If order_by_index is present, sort the selection by order_by_index
        if let Some(ord) = order_by_index {
            selection = sort_by_index(selection,ord);
        }

        // Initialize result to an empty vector
        let mut result: Vec<CosmosRustBotValue> = Vec::new();

        // Filter the entries in entry_store and add the matching entries to result
        for item in selection.iter().map(|x| self.0.entry_store.0.db.get(x).map(|x| x.map(|y| y.to_vec().try_into().unwrap())).unwrap_or(None::<CosmosRustBotValue>)) {
            // Map the selection to their corresponding entries, and iterate through them
            if let Some(entry) = item {
                // If the entry exists, check if it matches the filter
                if filter.is_empty() ||
                    filter
                        // Iterate through each filter condition
                        .iter().fold(false, |or, f| or || f.iter().fold(true, |sum, (k, v)|
                        {
                            let val = entry.get(k);

                            // If the value for the key doesn't exist, return false
                            if val == serde_json::Value::Null {
                                false
                            }else{
                                // Compare the value for the key against the filter value
                                if v.as_str() == "any" {
                                    // If the filter value is "any", always return true
                                    sum
                                } else if val.is_number() {
                                    // If the value is a number, compare it numerically
                                    &val.to_string() == v && sum
                                } else if let Some(s) = val.as_str() {
                                    // If the value is a string, compare it as a string
                                    s == v.as_str() && sum
                                } else {
                                    // Otherwise, the value type is not supported
                                    false
                                }
                            }
                        }
                    ))
                {
                    result.push(entry);
                }
            }
        }
        if let Some(l) = limit {
            result = result.into_iter().take(l).collect();
        }
        result
    }

    fn subscribe_unsubscribe_for_user(&mut self, query_result: &Vec<CosmosRustBotValue>, query_part: &EntriesQueryPart, settings_part: &SettingsPart) {

        if let Some(user_hash) = settings_part.user_hash {
            let subscribe = settings_part.subscribe.unwrap_or(false);
            let unsubscribe = settings_part.unsubscribe.unwrap_or(false);
            if subscribe || unsubscribe {
                let s_key = Subscription::get_key_for_entries_query(query_part);
                match self.0.subscription_store.0.get(&s_key) {
                    Ok(Some(s)) => {
                        if let CosmosRustBotValue::Subscription(mut s) = s.to_vec().try_into().unwrap() {
                            if subscribe {
                                s.add_user_hash(user_hash);
                                s.action = SubscriptionAction::AddUser;

                                let value: Vec<u8> = CosmosRustBotValue::Subscription(s).try_into().unwrap();
                                self.0.subscription_store.0.insert(s_key, value)
                                    .ok();
                            } else if unsubscribe {
                                if s.user_list.len() <= 1 {
                                    self.0.subscription_store.0.remove(&s_key).ok();
                                } else {
                                    s.remove_user_hash(user_hash);
                                    s.action = SubscriptionAction::RemoveUser;

                                    let value: Vec<u8> = CosmosRustBotValue::Subscription(s).try_into().unwrap();
                                    self.0.subscription_store.0.insert(s_key, value)
                                        .ok();
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        if !unsubscribe && subscribe {
                            let mut s = Subscription {
                                action: SubscriptionAction::Created,
                                query: QueryPart::EntriesQueryPart(query_part.clone()),
                                user_list: HashSet::new(),
                                list: Vec::new(),
                            };
                            s.add_user_hash(user_hash);
                            for e in query_result {
                                s.list.push(e.key());
                            }

                            let value: Vec<u8> = CosmosRustBotValue::Subscription(s).try_into().unwrap();
                            self.0.subscription_store.0.insert(s_key, value)
                                .ok();
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }

    fn register_and_get_token_for_user(&mut self, settings_part: &SettingsPart) -> Vec<CosmosRustBotValue> {

        if let Some(user_hash) = settings_part.user_hash {
            if let Some(true) = settings_part.register {
                let generate_token = || {
                    let mut rng = thread_rng();
                    rng.gen::<u64>()
                };

                let item = CosmosRustBotValue::Registration(Registration {
                    token: generate_token(),
                    user_hash,
                });
                let key = item.key();
                let value: Vec<u8> = item.try_into().unwrap();

                self.0.subscription_store.0.insert(key, value)
                    .ok();
            }

            let key = Registration::get_key_for_user_hash(user_hash);

            return match self.0.subscription_store.0.get(key) {
                Err(_e) => {
                    vec![]
                }
                Ok(None) => {
                    vec![]
                }
                Ok(Some(v)) => {
                    let result: CosmosRustBotValue = v.to_vec().try_into().unwrap();
                    vec![result]
                }
            }

        }
        vec![]
    }

    fn verify_auth_token(&mut self, query_part: &AuthQueryPart) -> Vec<CosmosRustBotValue> {

        let key = Registration::get_key_for_user_hash(query_part.user_hash);

        return match self.0.subscription_store.0.get(key) {
            Err(_e) => {
                vec![CosmosRustBotValue::Authorization(Authorization{ is_authorized: false, user_hash: query_part.user_hash })]
            }
            Ok(None) => {
                vec![CosmosRustBotValue::Authorization(Authorization{ is_authorized: false, user_hash: query_part.user_hash })]
            }
            Ok(Some(v)) => {
                let result: CosmosRustBotValue = v.to_vec().try_into().unwrap();

                match result {
                    CosmosRustBotValue::Registration(reg) => {
                        vec![CosmosRustBotValue::Authorization(Authorization{ is_authorized: reg.token == query_part.token, user_hash: query_part.user_hash })]
                    }
                    _ => {
                        vec![CosmosRustBotValue::Authorization(Authorization{ is_authorized: false, user_hash: query_part.user_hash })]
                    }
                }
            }
        }
    }

    fn opt_unsubscribe_and_get_subscriptions_for_user(&mut self, _query_part: &SubscriptionsQueryPart, settings_part: &SettingsPart) -> Vec<CosmosRustBotValue> {
        let mut res: Vec<CosmosRustBotValue> = Vec::new();

        if let Some(user_hash) = settings_part.user_hash {
            let mut r = self.0.subscription_store.0.db.scan_prefix(&Subscription::get_prefix()[..]);
            while let Some(Ok(item)) = r.next() {
                let val = item.1.to_vec().try_into().unwrap();
                match &val {
                    CosmosRustBotValue::Subscription(subscription) => {
                        if subscription.contains_user_hash(user_hash) {
                            if settings_part.unsubscribe.unwrap_or(false) {
                                let mut new_subscription = subscription.clone();
                                new_subscription.remove_user_hash(user_hash);
                                new_subscription.action = SubscriptionAction::RemoveUser;
                                let new_val = CosmosRustBotValue::Subscription(new_subscription);
                                let key = new_val.key();
                                let value: Vec<u8> = new_val.try_into().unwrap();
                                self.0.subscription_store.0.db.insert(key,value).ok();
                            }
                            res.push(val);
                        }
                    }
                    _ => {}
                }
            }
        }
        res
    }

}


pub fn inner_join_vec(list: &mut Vec<Vec<Vec<u8>>>) {
    if list.len() > 1 {
        let to_check = list.drain(1..).collect::<Vec<Vec<Vec<u8>>>>();
        list[0].retain(|x| to_check.iter().fold(true, |sum, list_to_check| { list_to_check.contains(&x) && sum}));
    }
}

pub fn sort_by_index(list: Vec<Vec<u8>>, order_by: Vec<Vec<u8>>) -> Vec<Vec<u8>>  {
    let mut ordered: Vec<Vec<u8>> = Vec::new();
    let mut unknown: Vec<Vec<u8>> = Vec::new();
    for key in order_by {
        if list.contains(&key) {
            ordered.push(key);
        }else{
            unknown.push(key);
        }
    }
    ordered.append(&mut unknown);
    ordered
}