use std::collections::HashMap;
use cosmos_rust_package::api::custom::query::gov::{ProposalExt};
use crate::utils::entry::*;
use strum::IntoEnumIterator;
use cosmos_rust_package::api::custom::query::gov::ProposalTime;
use crate::utils::response::{ResponseResult,BlockchainQuery};


pub fn governance_proposal_notifications(maybes: &HashMap<String, Maybe<ResponseResult>>) -> Vec<CosmosRustBotValue> {

    let mut view: Vec<CosmosRustBotValue> = Vec::new();
    list_latest_with(&mut view, maybes);

    CosmosRustBotValue::add_index(&mut view,"proposal_id","proposal_id");
    // add index for timestamps
    ProposalTime::iter().for_each(|time| {
        let k = format!("proposal_{}",time.to_string().to_lowercase());
        CosmosRustBotValue::add_index(&mut view,k.as_str(),k.as_str());
    });

    CosmosRustBotValue::add_variants_of_memberships(&mut view, vec!["proposal_blockchain","proposal_status","proposal_type"]);
    view
}

fn list_latest_with(view: &mut Vec<CosmosRustBotValue>, maybes: &HashMap<String, Maybe<ResponseResult>>) {
    let mut proposals: Vec<(&ProposalExt,String,i64)> = Vec::new();
    maybes.iter().for_each(|(key, y)| {
        if let Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(gov_proposals))), timestamp } = y {
           proposals.append(&mut gov_proposals.into_iter().map(|x| (x,key.to_string(),timestamp.to_owned())).collect());
        }
    });

    if proposals.len() >0 {
        for (proposal,origin,timestamp) in proposals.iter() {

            let mut data  = serde_json::json!({
                "proposal_id": proposal.proposal.proposal_id.to_string(),
                "proposal_blockchain": proposal.blockchain_name.to_string(),
                "proposal_status": proposal.status.to_string(),
                "proposal_type": proposal.content.to_string()
            });
            ProposalTime::iter().for_each(|time| {
                match (proposal.time(&time), time.to_string()) {
                    (Some(t), time_key) => {
                        data.as_object_mut().unwrap().insert(format!("proposal_{}", time_key.to_lowercase()), serde_json::json!(t.seconds/*{"seconds":t.seconds,"nanos":t.nanos}*/));
                    },
                    _ => {}
                }
            });

            view.push(
                CosmosRustBotValue::Entry(Entry::Value(Value{
                    timestamp: timestamp.to_owned(),
                    origin: origin.to_owned(),
                    summary: proposal.custom_display(),
                    custom_data: data.to_string()
                })));
        }
    }
}