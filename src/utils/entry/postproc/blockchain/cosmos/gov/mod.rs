use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::iter;
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::{Maybe, Entry, EntryValue};
use strum::IntoEnumIterator;
use cosmos_rust_package::api::custom::query::gov::ProposalTime;
use crate::utils::response::{ResponseResult,BlockchainQuery};


pub fn governance_proposal_notifications(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<Entry> {

    let mut view: HashMap<u64, Entry> = HashMap::new();
    list_latest_by(&mut view, maybes,  Some(ProposalTime::LatestTime));
    let mut view: Vec<Entry> = view.into_iter().map(|(_id, x)| x).collect();
    view.sort_by(|a, b| {
        match (a,b) {
            (Entry{value: EntryValue::Value(x), ..},Entry{ value: EntryValue::Value(y),..}) => {
                let xx = x.get("ranks").unwrap().as_array().unwrap().into_iter().filter(|c| c["order_by_LatestTime"]!=serde_json::Value::Null).map(|c| c["order_by_LatestTime"].as_u64().unwrap()).collect::<Vec<u64>>()[0];
                let yy = y.get("ranks").unwrap().as_array().unwrap().into_iter().filter(|c| c["order_by_LatestTime"]!=serde_json::Value::Null).map(|c| c["order_by_LatestTime"].as_u64().unwrap()).collect::<Vec<u64>>()[0];
                xx.cmp(&yy)
            },
            _ => {
                panic!()
            }
        }
    });
    view
}

fn list_latest_by(view: &mut HashMap<u64,Entry>, maybes: &HashMap<String, Maybe<ResponseResult>>, time: Option<ProposalTime>) {
    let mut proposals: Vec<(&ProposalExt,String,i64)> = Vec::new();
    maybes.iter().for_each(|(key, y)| {
        if let Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(gov_proposals))), timestamp } = y {
           let relevant_proposals: Vec<&ProposalExt> = gov_proposals.iter().filter(|x| {
               if time.is_none() || x.time(time.as_ref().unwrap().clone()).is_some() {
                   true
               }else {
                   false
               }
           }).collect();
           proposals.append(&mut relevant_proposals.into_iter().map(|x| (x,key.to_string(),timestamp.to_owned())).collect());
        }
    });

    if proposals.len() >0 {
        if let Some(ref t) = time {
            proposals.sort_by(|a, b| a.0.time(t.clone()).as_ref().unwrap().seconds.cmp(&b.0.time(t.clone()).as_ref().unwrap().seconds));
        }
        for (i,(proposal,key,timestamp)) in proposals.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            proposal.hash(&mut hasher);
            let hash = hasher.finish();
            let rank = serde_json::json!({format!("order_by_{}", time.as_ref().map(|x| x.to_string()).unwrap_or("None".to_string())).to_string(): i});
            if !view.contains_key(&hash) {
                view.insert(hash,Entry {
                    timestamp: timestamp.to_owned(),
                    key: key.to_owned(),
                    value: EntryValue::Value(serde_json::json!({"info": /*format!("{:?}",proposal)*/ proposal.custom_display(),"id":proposal.proposal.proposal_id,"ranks": vec![rank]}))
                });
            }else{
                let item = view.get_mut(&hash).unwrap();
                if let EntryValue::Value(ref mut val) = item.value {
                    val.get_mut("ranks").unwrap().as_array_mut().unwrap().push(rank);
                }
            }
        }
    }


}