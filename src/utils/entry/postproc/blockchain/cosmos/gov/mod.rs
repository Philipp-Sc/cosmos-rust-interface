use std::collections::HashMap;
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::*;
use strum::IntoEnumIterator;
use cosmos_rust_package::api::custom::query::gov::ProposalTime;
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::response::{ResponseResult, BlockchainQuery, FraudClassification};

use serde::{Deserialize,Serialize};


/// # Governance Proposal Notifications
///
/// This method generates the entries for the governance proposal notifications.
///
///
pub fn governance_proposal_notifications(task_store: &TaskMemoryStore) -> Vec<CosmosRustBotValue> {

    let mut view: Vec<CosmosRustBotValue> = Vec::new();
    add_proposals(&mut view, task_store);

    CosmosRustBotValue::add_index(&mut view,"proposal_id","proposal_id");
    // add index for timestamps
    ProposalTime::iter().for_each(|time| {
        let k = format!("proposal_{}",time.to_string());
        CosmosRustBotValue::add_index(&mut view,k.as_str(),k.as_str());
    });

    CosmosRustBotValue::add_variants_of_memberships(&mut view, vec!["proposal_blockchain","proposal_status","proposal_type"]);
    view
}
/// # Adds proposals
///
/// This function will add proposals from all the blockchains.
///
fn add_proposals(view: &mut Vec<CosmosRustBotValue>, task_store: &TaskMemoryStore) {

    //let mut proposals_for_csv: Vec<ProposalData> = Vec::new();

    for (key, y) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        if let Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(gov_proposals))), timestamp } = y {

            for (mut proposal,origin,timestamp) in gov_proposals.into_iter().map(|x| (x, key.to_string(), timestamp.to_owned())) {

                let fraud_classification = match task_store.get::<ResponseResult>(&format!("FRAUD_DETECTION_{}",proposal.title_and_description_to_hash()),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::FraudClassification(FraudClassification{title, description, text, fraud_prediction })), timestamp }) => {
                        Some(fraud_prediction)
                    }
                    Err(_) => {None}
                    _ => {None}
                };


                let data =  ProposalData {
                        proposal_link: proposal.governance_proposal_link(),
                        proposal_clickbait: proposal.proposal_clickbait(fraud_classification),
                        proposal_quick_summary: format!("Coming soon"),
                        proposal_content: proposal.proposal_content(),
                        proposal_state: proposal.proposal_state(),
                        proposal_details: proposal.proposal_details(fraud_classification),
                        proposal_blockchain: proposal.blockchain_name.to_string(),
                        proposal_status: proposal.status.to_string(),
                        proposal_id: proposal.proposal().map(|x| x.proposal_id),
                        proposal_type: proposal.content().map(|x| x.to_string()),
                        proposal_SubmitTime: proposal.time(&ProposalTime::SubmitTime).map(|t| t.seconds),
                        proposal_DepositEndTime: proposal.time(&ProposalTime::DepositEndTime).map(|t| t.seconds),
                        proposal_VotingStartTime: proposal.time(&ProposalTime::VotingStartTime).map(|t| t.seconds),
                        proposal_VotingEndTime: proposal.time(&ProposalTime::VotingEndTime).map(|t| t.seconds),
                        proposal_LatestTime: proposal.time(&ProposalTime::LatestTime).map(|t| t.seconds),
                        proposal_custom_display: proposal.proposal_details(None),
                        proposal_title: proposal.get_title_and_description().0,
                        proposal_description: proposal.get_title_and_description().1,
                        proposal_vetoed: proposal.proposal().map(|x| x.final_tally_result.map(|y| y.no_with_veto.parse::<f64>().unwrap_or(0f64) > y.yes.parse::<f64>().unwrap_or(0f64) && y.no_with_veto.parse::<f64>().unwrap_or(0f64) > y.no.parse::<f64>().unwrap_or(0f64))).flatten().unwrap_or(false)
                };

                if fraud_classification.is_some() || (proposal.status!=ProposalStatus::StatusVotingPeriod && proposal.status!=ProposalStatus::StatusDepositPeriod) {
                    view.push(
                        CosmosRustBotValue::Entry(Entry::Value(Value {
                            timestamp: timestamp.to_owned(),
                            origin: origin.to_owned(),
                            custom_data: CustomData::ProposalData(data)
                        })));
                }


                // proposals_for_csv.push(data);
            }
        }
    }
/*
    let mut wtr = csv::Writer::from_path("./tmp/proposals.csv").unwrap();

    wtr.write_record(&["text", "label","type"]).unwrap();
    for each in proposals_for_csv {
        if each.proposal_status =="StatusRejected" || each.proposal_status =="StatusPassed" {
            wtr.write_record(&[&each.proposal_custom_display.as_str(), &if each.proposal_vetoed {"1"}else{"0"},"custom_display"]).unwrap();
            wtr.write_record(&[&each.proposal_description.as_str(), &if each.proposal_vetoed {"1"}else{"0"},"description"]).unwrap();
            wtr.write_record(&[&format!("{}\n\n{}",each.proposal_title,each.proposal_description).as_str(), &if each.proposal_vetoed {"1"}else{"0"},"title_and_description"]).unwrap();
            //wtr.serialize(each).unwrap();
        }
    }*/
}