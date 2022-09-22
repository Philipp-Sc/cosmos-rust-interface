use crate::utils::entry::*;

pub mod socket;


pub fn handle_query_sled_db(db: &sled::Db, query: &UserQuery) -> Vec<CosmosRustBotValue> {

    match &query.query_part {
        QueryPart::EntriesQueryPart(query_part) => {
            query_entries_sled_db(db,query_part, &query.settings_part)
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
            let val = CosmosRustBotValue::from(item.1.to_vec());
            match &val {
                CosmosRustBotValue::Subscription(subscription) => {
                    let mut new_subscription = subscription.clone();
                    if new_subscription.contains_user_hash(user_hash) {
                        if settings_part.unsubscribe.unwrap_or(false) {
                            new_subscription.remove_user_hash(user_hash);
                            let new_val = CosmosRustBotValue::Subscription(new_subscription);
                            db.insert(new_val.key(), new_val.value()).ok();
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

pub fn query_entries_sled_db(db: &sled::Db, query_part: &EntriesQueryPart, settings_part: &SettingsPart) -> Vec<CosmosRustBotValue> {

    let mut filter: Vec<String> = query_part.filter.iter()
        .map(|(k, v)| format!("{}_{}", k, v.to_lowercase()))
        .collect();

    let order_by: Option<&str> = Some(&query_part.order_by);
    let limit: Option<usize> = Some(query_part.limit);

    let mut indices_list: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut order_by_index: Option<Vec<Vec<u8>>> = None;
    //println!("{:?}", &order_by_index.map(|x| x.len()));
    let mut r = db.scan_prefix(&Index::get_prefix()[..]);
    while let Some(Ok(item)) = r.next() {
        let val: CosmosRustBotValue = CosmosRustBotValue::from(item.1.to_vec());
        //print!("{:?}", val.try_get("name"));
        match val {
            CosmosRustBotValue::Index(index) => {
                //println!("{:?}", index.name);
                let index_name_in_filter = filter.contains(&index.name);
                if query_part.indices.contains(&index.name) || index_name_in_filter {
                    // remember if index for filter was used
                    if index_name_in_filter {
                        filter.retain(|x| x != &index.name);
                    }
                    indices_list.push(index.list.clone());
                }
                if let Some(ord) = order_by {
                    if &index.name == &ord {
                        order_by_index = Some(index.list.clone());
                    }
                }
            }
            _ => {}
        }
    }
    //println!("indices list len: {}", indices_list.len());
    let mut section: Vec<&Vec<u8>> = Vec::new();
    if indices_list.len() > 1 {
        for each in &indices_list[0] {
            let mut c: bool = true;
            for i in 1..indices_list.len() {
                if !indices_list[i].contains(&each) {
                    c = false;
                    break;
                }
            }
            if c {
                section.push(&each);
            }
        }
    } else if indices_list.len() == 1 {
        section = indices_list[0].iter().map(|x| x).collect();
    }
    //println!("section list len: {}", section.len());

    let mut res: Vec<CosmosRustBotValue> = Vec::new();
    if let Some(ord) = order_by_index {
        for each in &ord {
            if section.contains(&each) {
                if let Ok(Some(t)) = db
                    .get(each)
                    .map(|x| x.map(|y| CosmosRustBotValue::from(y.to_vec())))
                {
                    res.push(t);
                }
            }
        }
    } else {
        for each in section {
            if let Ok(Some(t)) = db
                .get(each)
                .map(|x| x.map(|y| CosmosRustBotValue::from(y.to_vec())))
            {
                res.push(t);
            }
        }
    }
    let mut final_res: Vec<CosmosRustBotValue> = Vec::new();
    for entry in res {
        //println!("{:?}", &entry);
        if filter.len() == 0
            || query_part.filter
                .iter()
                .filter(|(k, v)| filter.contains(&format!("{}_{}", k, v.to_lowercase())))
                .fold(true, |sum, (k, v)| match entry.try_get(k) {
                    None => false,
                    Some(val) => {
                        //println!("EntryValue: {:?}, FilterValue: {:?}, FilterKey: {:?}",&val, v, k );
                        if let Some(s) = val.as_str() {
                            (v.as_str() == "any" || s == v.as_str()) && sum
                        } else if val.is_number() {
                            format!("{}", val).as_str() == v.as_str() && sum
                        } else {
                            false
                        }
                    }
                })
        {
            final_res.push(entry);
        }
    }
    //println!("{:?}", &final_res);
    if let Some(l) = limit {
        final_res = final_res.into_iter().take(l).collect();
    }

    let subscribe = settings_part.subscribe.unwrap_or(false);
    let update_subscription = settings_part.update_subscription.unwrap_or(false);
    let unsubscribe = settings_part.unsubscribe.unwrap_or(false);
    if subscribe || update_subscription || unsubscribe {
        let s_key = Subscription::get_key_for_entries_query(query_part);
        match db.get(&s_key) {
            Ok(Some(s)) => {
                let mut s = match CosmosRustBotValue::from(s.to_vec()) {
                    CosmosRustBotValue::Subscription(t) => t,
                    _ => {
                        panic!();
                    }
                };
                if subscribe {
                    if let Some(user_hash) = settings_part.user_hash {
                        s.add_user_hash(user_hash);
                    }
                }
                let mut added_or_removed_items = false;
                if update_subscription {
                    let final_res_keys =
                        final_res.iter().map(|x| x.key()).collect::<Vec<Vec<u8>>>();
                    for e in &final_res_keys {
                        if !s.list.contains(&e) {
                            added_or_removed_items = true;
                            s.list.push(e.clone());
                        }
                    }
                    let len = s.list.len();
                    s.list.retain(|x| final_res_keys.contains(x));
                    if len != s.list.len() {
                        added_or_removed_items = true;
                    }
                }
                let rm = s.user_list.len() <= 1;
                if unsubscribe {
                    if let Some(user_hash) = settings_part.user_hash {
                        if rm {
                            db.remove(&s_key).ok();
                        } else {
                            s.remove_user_hash(user_hash);
                        }
                    }
                }
                if subscribe
                    || (update_subscription && added_or_removed_items)
                    || (!rm && unsubscribe)
                {
                    db.insert(s_key, CosmosRustBotValue::Subscription(s).value())
                        .ok();
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
                        for e in final_res.iter() {
                            s.list.push(e.key());
                        }
                        db.insert(s_key, CosmosRustBotValue::Subscription(s).value())
                            .ok();
                    }
                }
            }
            Err(_) => {}
        }
    }
    final_res
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
