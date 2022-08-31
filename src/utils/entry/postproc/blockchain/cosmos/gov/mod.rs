use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::iter;
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use cosmos_rust_package::api::custom::query::gov::{ProposalExt};
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
                let v = serde_json::json!({});
                let xx = x.get("order_by").unwrap_or(&v).as_object().unwrap()["LatestTime"].as_u64().unwrap_or(0);
                let yy = y.get("order_by").unwrap_or(&v).as_object().unwrap()["LatestTime"].as_u64().unwrap_or(0);
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
        for (i,(proposal,origin,timestamp)) in proposals.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            proposal.hash(&mut hasher);
            let hash = hasher.finish();
            let key = format!("{}", time.as_ref().map(|x| x.to_string()).unwrap_or("None".to_string())).to_string();
            let value = i;
            let rank = serde_json::json!({"id": proposal.proposal.proposal_id, key.to_owned(): value.to_owned()});

            if !view.contains_key(&hash) {
                let filter  = serde_json::json!({
                    "id": proposal.proposal.proposal_id.to_string(),
                    "blockchain": proposal.blockchain_name.to_string(),
                    "status": proposal.status.to_string(),
                    "type": proposal.content.to_string()
                });

                view.insert(hash,Entry {
                    timestamp: timestamp.to_owned(),
                    origin: origin.to_owned(),
                    value: EntryValue::Value(serde_json::json!({"info": /*format!("{:?}",proposal)*/ proposal.custom_display(),"where": filter,"order_by": rank}))
                });
            }else{
                let item = view.get_mut(&hash).unwrap();
                if let EntryValue::Value(ref mut val) = item.value {
                    val.get_mut("order_by").unwrap().as_object_mut().unwrap().insert(key,serde_json::json!(value));
                }
            }
        }
    }


}