use crate::utils::entry::*;

pub mod socket;


pub enum DatabaseVariant<'a> {
    Sled(&'a sled::Db),
    Vec(&'a Vec<CosmosRustBotValue>),
}

impl DatabaseVariant<'_> {
    pub fn get_indices(&self) -> Vec<Index>
    {
        return match &self {
            DatabaseVariant::Sled(db) => {
                let mut result: Vec<Index> = Vec::new();
                let mut iter = db.scan_prefix(&Index::get_prefix()[..]);
                while let Some(Ok(item)) = iter.next() {
                    match item.1.to_vec().try_into().unwrap(){
                        CosmosRustBotValue::Index(index) => {
                            result.push(index);
                        },
                        _ => { }
                    }
                }
                result
            }
            DatabaseVariant::Vec(vec) => {
                vec.iter().map(|x| if let CosmosRustBotValue::Index(index) = x { Some(index) } else { None }).filter(|x| x.is_some()).map(|x| x.unwrap().clone()).collect()
            }
        }
    }
    pub fn get_entry(&self, key: &Vec<u8>) -> Option<CosmosRustBotValue>
    {
        return match &self {
            DatabaseVariant::Sled(db) => {
                db.get(key).map(|x| x.map(|y| y.to_vec().try_into().unwrap())).unwrap_or(None)
            }
            DatabaseVariant::Vec(vec) => {
                for item in vec.iter() {
                    if &item.key() == key {
                        return Some(item.clone());
                    }
                }
                None
            }
        };
    }
}


pub fn handle_query_sled_db(db: &sled::Db, query: &UserQuery) -> Vec<CosmosRustBotValue> {

    match &query.query_part {
        QueryPart::EntriesQueryPart(query_part) => {
            query_entries_sled_db(DatabaseVariant::Sled(db),query_part, &query.settings_part)
        },
        QueryPart::SubscriptionsQueryPart(query_part) => {
            query_subscriptions_sled_db(db,query_part, &query.settings_part)
        }
    }
}

pub fn query_subscriptions_sled_db(db: &sled::Db, _query_part: &SubscriptionsQueryPart, settings_part: &SettingsPart) -> Vec<CosmosRustBotValue> {
    let mut res: Vec<CosmosRustBotValue> = Vec::new();

    if let Some(user_hash) = settings_part.user_hash {
        let mut r = db.scan_prefix(&Subscription::get_prefix()[..]);
        while let Some(Ok(item)) = r.next() {
            let val = item.1.to_vec().try_into().unwrap();
            match &val {
                CosmosRustBotValue::Subscription(subscription) => {
                    let mut new_subscription = subscription.clone();
                    if new_subscription.contains_user_hash(user_hash) {
                        if settings_part.unsubscribe.unwrap_or(false) {
                            new_subscription.remove_user_hash(user_hash);
                            let new_val = CosmosRustBotValue::Subscription(new_subscription);
                            let key = new_val.key();
                            let value: Vec<u8> = new_val.try_into().unwrap();
                            db.insert(key,value).ok();
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

fn inner_join_vec(list: &mut Vec<Vec<Vec<u8>>>) {
    if list.len() > 1 {
        let to_check = list.drain(1..).collect::<Vec<Vec<Vec<u8>>>>();
        list[0].retain(|x| to_check.iter().fold(true, |sum, list_to_check| { list_to_check.contains(&x) && sum}));
    }
}

fn sort_by_index(list: Vec<Vec<u8>>, order_by: Vec<Vec<u8>>) -> Vec<Vec<u8>>  {
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

pub fn query_entries_sled_db(database_variant: DatabaseVariant, query_part: &EntriesQueryPart, settings_part: &SettingsPart) -> Vec<CosmosRustBotValue> {

    let mut filter = query_part.filter.clone();

    let order_by: Option<&str> = Some(&query_part.order_by);
    let limit: Option<usize> = Some(query_part.limit);

    let mut indices_list: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut order_by_index: Option<Vec<Vec<u8>>> = None;
    //println!("{:?}", &order_by_index.map(|x| x.len()));
    for index in database_variant.get_indices() {
        //print!("{:?}", val.try_get("name"));
        //println!("{:?}", index.name);

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

        if let Some(ord) = order_by {
            if &index.name == &ord {
                order_by_index = Some(index.list.clone());
            }
        }
    }
    inner_join_vec(&mut indices_list);

    let mut selection: Vec<Vec<u8>> = Vec::new();
    if indices_list.len()>0 {
        selection.append(&mut indices_list[0]);
    }

    if let Some(ord) = order_by_index {
        selection = sort_by_index(selection,ord);
    }

    let mut result: Vec<CosmosRustBotValue> = Vec::new();

    for item in selection.iter().map(|x| database_variant.get_entry(x)) {
        if let Some(entry) = item {
            if filter.is_empty() ||
                filter
                .iter().fold(false, |or, f| or || f.iter().fold(true, |sum, (k, v)|
                    {
                        let val = entry.get(k);
                        if val == serde_json::Value::Null {
                            //println!("Key: {:?}", k );
                            false
                        }else{
                            //println!("{:?}==?{:?}, Key: {:?}",&val, v, k );
                            if v.as_str() == "any" {
                                sum
                            } else if val.is_number() {
                                // gt, lt, eq,
                                /*
                                if v.contains("eq "){
                                    let compare = v.replace("eq ","");
                                    &val.to_string() == &compare && sum
                                }else if v.contains("lt "){
                                    let compare = v.replace("lt ","");
                                    &val.as_f64().unwrap_or(0f64) < &compare.parse::<f64>().unwrap_or(0f64) && sum
                                }else if v.contains("gt "){
                                    let compare = v.replace("gt ","");
                                    &val.as_f64().unwrap_or(0f64) > &compare.parse::<f64>().unwrap_or(0f64) && sum
                                }else {*/
                                &val.to_string() == v && sum
                                //}
                            } else if let Some(s) = val.as_str() {
                                s == v.as_str() && sum
                            } else {
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
    match database_variant {
        DatabaseVariant::Sled(db) => query_subscribe_unsubscribe_sled_db(db,&result,query_part,settings_part),
        _ => {}
    }
    result
}

pub fn query_subscribe_unsubscribe_sled_db(db: &sled::Db, query_result: &Vec<CosmosRustBotValue>, query_part: &EntriesQueryPart, settings_part: &SettingsPart) {

    let subscribe = settings_part.subscribe.unwrap_or(false);
    let unsubscribe = settings_part.unsubscribe.unwrap_or(false);
    if subscribe || unsubscribe {
        let s_key = Subscription::get_key_for_entries_query(query_part);
        match db.get(&s_key) {
            Ok(Some(s)) => {
                if let CosmosRustBotValue::Subscription(mut s) = s.to_vec().try_into().unwrap() {
                    if subscribe {
                        if let Some(user_hash) = settings_part.user_hash {
                            s.add_user_hash(user_hash);

                            let value: Vec<u8> = CosmosRustBotValue::Subscription(s).try_into().unwrap();
                            db.insert(s_key, value)
                                .ok();
                        }
                    } else if unsubscribe {
                        if let Some(user_hash) = settings_part.user_hash {
                            if s.user_list.len() <= 1 {
                                db.remove(&s_key).ok();
                            } else {
                                s.remove_user_hash(user_hash);

                                let value: Vec<u8> = CosmosRustBotValue::Subscription(s).try_into().unwrap();
                                db.insert(s_key, value)
                                    .ok();
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                if !unsubscribe && subscribe {
                    if let Some(user_hash) = settings_part.user_hash {
                        let mut s = Subscription {
                            query: QueryPart::EntriesQueryPart(query_part.clone()),
                            user_list: HashSet::new(),
                            list: Vec::new(),
                        };
                        s.add_user_hash(user_hash);
                        for e in query_result {
                            s.list.push(e.key());
                        }

                        let value: Vec<u8> = CosmosRustBotValue::Subscription(s).try_into().unwrap();
                        db.insert(s_key, value)
                            .ok();
                    }
                }
            }
            Err(_) => {}
        }
    }
}

pub fn update_subscription(db: &Vec<CosmosRustBotValue>, subscription: &mut Subscription) -> anyhow::Result<()> {

    if let QueryPart::EntriesQueryPart(query_part) = &subscription.query
    {
        let query_result = query_entries_sled_db(DatabaseVariant::Vec(db), query_part, &SettingsPart {
            subscribe: None,
            unsubscribe: None,
            user_hash: None
        });

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
        if added_items || removed_items
        {
            return Ok(())
        }
    }
    Err(anyhow::anyhow!("Error: Nothing to update"))
}

/*pub fn query_entries(entries: &Vec<CosmosRustBotValue>, filter: HashMap<String, String>, order_by: String, limit: usize) -> Vec<&CosmosRustBotValue> {
    let mut result: Vec<&CosmosRustBotValue> = entries.iter().filter(|item| {
        if let EntryValue::Value(ref val) = item.value {
            if val.get("where").is_some(){
                if let Some(filter_options) = val.get("where").unwrap().as_object() {
                    let res: bool = filter.iter().map(|(k,v)| {
                        filter_options.contains_key(k.as_str()) && (filter_options.get(k.as_str()).unwrap() == &serde_json::json!(v) || v == "any")
                    }).fold(true,|x, y| {x && y});
                    return  res;
                }
            }
        }
        return false
    }).collect();
    result.sort_by(|a, b| {
        match (a,b) {
            (CosmosRustBotValue{value: EntryValue::Value(x), ..},CosmosRustBotValue{ value: EntryValue::Value(y),..}) => {
                let v = serde_json::json!({});
                let xx = match &x["order_by"] {
                    serde_json::Value::Object(obj) => {
                        match &obj[&order_by] {
                            serde_json::Value::Number(val)  => {
                                val.as_u64().unwrap_or(0)
                            }
                            _ => { 0 as u64}
                        }
                    }
                    serde_json::Value::Null | _ => {
                        0 as u64
                    }
                };

                let yy = match &y["order_by"] {
                    serde_json::Value::Object(obj) => {
                        match &obj[&order_by] {
                            serde_json::Value::Number(val) => {
                                val.as_u64().unwrap_or(0)
                            }
                            serde_json::Value::Null | _ => { 0 as u64}
                        }
                    }
                    serde_json::Value::Null | _ => {
                        0 as u64
                    }
                };

                xx.cmp(&yy)
            },
            _ => {
                panic!()
            }
        }
    });
   result.into_iter().rev().take(limit).collect()
}*/
