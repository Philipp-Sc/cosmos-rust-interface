use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::iter;
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::{BlockchainQuery, ResponseResult};
use crate::utils::postproc::{Maybe, Entry, EntryValue};
use strum::IntoEnumIterator;
use cosmos_rust_package::api::custom::query::gov::ProposalTime;



pub fn governance_proposal_notifications(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<Entry> {

    // iterate over all items to filter by matching
    // ResponseResult::Blockchain(BlockchainQuery::GovProposals(Vec<ProposalExt>))


    let mut view: HashMap<u64, Entry> = HashMap::new();

    for blockchain in SupportedBlockchain::iter().map(|x| Some(x)).chain(iter::once(None)){
        for status in ProposalStatus::iter().map(|x| Some(x)).chain(iter::once(None)){
            for time in ProposalTime::iter().map(|x| Some(x)).chain(iter::once(None)){
                list_latest_by(&mut view, maybes, blockchain.clone(), status.clone(), time.clone());
            }
        }
    }
    let mut view: Vec<Entry> = view.into_iter().map(|(_id, x)| x).collect();
    view.sort_by(|a, b| {
        match (a,b) {
            (Entry{value: EntryValue::Value(x), ..},Entry{ value: EntryValue::Value(y),..}) => {
                let xx = x.get("ranks").unwrap().as_array().unwrap().into_iter().filter(|c| c["where_Any_Any_order_by_VotingStartTime"]!=serde_json::Value::Null).map(|c| c["where_Any_Any_order_by_VotingStartTime"].as_u64().unwrap()).collect::<Vec<u64>>()[0];
                let yy = y.get("ranks").unwrap().as_array().unwrap().into_iter().filter(|c| c["where_Any_Any_order_by_VotingStartTime"]!=serde_json::Value::Null).map(|c| c["where_Any_Any_order_by_VotingStartTime"].as_u64().unwrap()).collect::<Vec<u64>>()[0];
                xx.cmp(&yy)
            },
            _ => {
                panic!()
            }
        }
    });
    view
}

fn list_latest_by(view: &mut HashMap<u64,Entry>, maybes: &HashMap<String, Maybe<ResponseResult>>, blockchain: Option<SupportedBlockchain>, status: Option<ProposalStatus>, time: Option<ProposalTime>) {
    let mut proposals: Vec<(&ProposalExt,String,i64)> = Vec::new();
    maybes.iter().for_each(|(key, y)| {
        if let Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(gov_proposals))), timestamp } = y {
           let relevant_proposals: Vec<&ProposalExt> = gov_proposals.iter().filter(|x| {
               if (blockchain.is_none() || x.blockchain == blockchain.as_ref().unwrap().clone()) &&
                   (status.is_none() || x.status == status.as_ref().unwrap().clone()) &&
                   (time.is_none() || x.time(time.as_ref().unwrap().clone()).is_some()){
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
            proposals.sort_by(|a, b| a.0.time(t.clone()).as_ref().unwrap().nanos.cmp(&b.0.time(t.clone()).as_ref().unwrap().nanos));
        }
        for (i,(proposal,key,timestamp)) in proposals.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            proposal.hash(&mut hasher);
            let hash = hasher.finish();
            let rank = serde_json::json!({format!("where_{}_{}_order_by_{}", blockchain.as_ref().map(|x| x.to_string()).unwrap_or("Any".to_string()),status.as_ref().map(|x| x.to_string()).unwrap_or("Any".to_string()), time.as_ref().map(|x| x.to_string()).unwrap_or("Any".to_string())).to_string(): i});
            if !view.contains_key(&hash) {
                view.insert(hash,Entry {
                    timestamp: timestamp.to_owned(),
                    key: key.to_owned(),
                    value: EntryValue::Value(serde_json::json!({"data": format!("{:?}",proposal.content()),"ranks": vec![rank]}))
                });
            }else{
                let item = view.get_mut(&hash).unwrap();
                if let EntryValue::Value(ref mut val) = item.value {
                    val.get_mut("ranks").unwrap().as_array_mut().unwrap().push(rank);
                }
            }
            // #1 without index it makes no sense to store all these duplicates?!
            // #2 storing to much data, instead store only the indices of the different ordering!
            // add value attr: format!("rank_by_{}_{}_{}", blockchain, status, time) : i64
        }
    }


}