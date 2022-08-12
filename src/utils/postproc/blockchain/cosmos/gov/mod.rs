use std::collections::HashMap;
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


    let mut view: Vec<Entry>= Vec::new();

    for blockchain in SupportedBlockchain::iter().map(|x| Some(x)).chain(iter::once(None)){
        for status in ProposalStatus::iter().map(|x| Some(x)).chain(iter::once(None)){
            for time in ProposalTime::iter().map(|x| Some(x)).chain(iter::once(None)){
                view.append(&mut list_latest_by(maybes, blockchain.clone(), status.clone(), time.clone()));
            }
        }
    }
    view
}

fn list_latest_by(maybes: &HashMap<String, Maybe<ResponseResult>>, blockchain: Option<SupportedBlockchain>, status: Option<ProposalStatus>, time: Option<ProposalTime>) -> Vec<Entry> {
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


    let mut view: Vec<Entry> = Vec::new();
    if proposals.len() >0 {
        if let Some(t) = time {
            proposals.sort_by(|a, b| a.0.time(t.clone()).as_ref().unwrap().nanos.cmp(&b.0.time(t.clone()).as_ref().unwrap().nanos));
        }
        for (proposal,key,timestamp) in proposals {
            view.push(Entry {
                timestamp: timestamp.to_owned(),
                key: key.to_owned(),
                value: EntryValue::Json(serde_json::json!({"data": format!("{:?}",proposal.content())}).to_string())
            });
            // #1 without index it makes no sense to store all these duplicates?!
            // #2 storing to much data, instead store only the indices of the different ordering!
            // add value attr: format!("rank_by_{}_{}_{}", blockchain, status, time) : i64
        }
    }


    view
}