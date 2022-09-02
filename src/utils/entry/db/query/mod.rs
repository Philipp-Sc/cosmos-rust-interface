use std::collections::HashMap;
use crate::utils::entry::*;

pub fn query_sled_db(db: &sled::Db, query: serde_json::Value) -> Vec<CosmosRustBotValue> {
    // serde_json::json!({"indices":vec!["task_meta_data"],"filter": filter, "order_by": order_by, "limit":limit})
    let empty: Vec<serde_json::Value> = Vec::new();
    let indices = query.get("indices").map(|x| x.as_array().unwrap_or(&empty)).unwrap_or(&empty).iter().map(|x| x.as_str().unwrap_or("")).collect::<Vec<&str>>();

    let filter_k_v_pair: Vec<(String,String)> = match query.get("filter") {
        Some(filter) => {
            match filter.as_object() {
                Some(obj) => {
                    obj.iter().filter(|(_, v)| v.as_str().is_some()).map(|(k,v)| (k.to_string(),v.as_str().unwrap().to_string())).collect()
                    },
                None => { Vec::new() }
            }
        },
        None => {Vec::new()}
    };
    let filter: Vec<String> = filter_k_v_pair.iter().map(|(k,v)| format!("{}_{}",k,v.to_lowercase())).collect();

    let order_by: Option<&str> = query.get("order_by").map(|x| x.as_str()).unwrap_or(None);

    let limit: Option<usize> = query.get("limit").map(|x| x.as_u64().map(|y| y as usize)).unwrap_or(None);


    let mut indices_list: Vec<Vec<Vec<u8>>> = Vec::new();
    let mut order_by_index: Option<Vec<Vec<u8>>> = None;
    let mut r = db.scan_prefix(&Index::get_prefix()[..]);
    while let Some(Ok(item)) = r.next() {
        let val: CosmosRustBotValue = CosmosRustBotValue::from(item.1.to_vec());
        //print!("{:?}", val.try_get("name"));
        match val {
            CosmosRustBotValue::Index(index) => {
                //println!("{:?}",index.name);
                if indices.contains(&index.name.as_str()) || filter.contains(&index.name) { // todo: reduce workload by remembering if index for filter was used
                    indices_list.push(index.list.clone());
                }
                if let Some(ord) = order_by {
                    if &index.name.as_str() == &ord {
                        order_by_index = Some(index.list.clone());
                    }
                }
            },
            _ => {}
        }
    }
    //println!("indices list len: {}",indices_list.len());
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
    }else if indices_list.len() == 1 {
        section = indices_list[0].iter().map(|x| x).collect();
    }
    //println!("section list len: {}",section.len());

    let mut res: Vec<CosmosRustBotValue> = Vec::new();
    if let Some(ord) = order_by_index {
        for each in &ord {
            if section.contains(&each) {
                if let Ok(Some(t)) = db.get(each).map(|x| x.map(|y| CosmosRustBotValue::from(y.to_vec()))) {
                    // here do filter value check
                    res.push(t);
                }
            }
        }
    }else {
        for each in section {
            if let Ok(Some(t)) = db.get(each).map(|x| x.map(|y| CosmosRustBotValue::from(y.to_vec()))) {
                // here do filter value check
                res.push(t);
            }
        }
    }
    if let Some(l) = limit {
        return res.into_iter().take(l).collect();
    }
    res

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