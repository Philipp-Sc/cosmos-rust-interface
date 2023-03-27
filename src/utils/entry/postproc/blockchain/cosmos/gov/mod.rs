use std::collections::HashMap;
use chrono::Utc;
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::*;
use strum::IntoEnumIterator;
use cosmos_rust_package::api::custom::query::gov::ProposalTime;
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::response::{ResponseResult, BlockchainQuery, FraudClassification, ProposalDataResult};

use serde::{Deserialize,Serialize};
use rust_openai_gpt_tools_socket_ipc::ipc::{OpenAIGPTResult, OpenAIGPTChatCompletionResult};
use crate::services::fraud_detection::get_key_for_fraud_detection;
use crate::services::gpt3::get_key_for_gpt3;


const PROPOSAL_DATA_RESULT: &str = "ProposalDataResult";

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

    let mut list_proposal_hash: Vec<u64> = if let Ok(Maybe { data: Ok(ResponseResult::ProposalDataResult(ProposalDataResult{list_proposal_hash: list})), timestamp}) = task_store.get(PROPOSAL_DATA_RESULT, &RetrievalMethod::Get){
        list
    }else{
        Vec::new()
    };

    for (key, y) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        if let Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(gov_proposals))), timestamp } = y {

            for (mut proposal,origin,timestamp) in gov_proposals.into_iter().map(|x| (x, key.to_string(), timestamp.to_owned())) {

                let proposal_hash = proposal.to_hash();

                let hash = proposal.id_title_and_description_to_hash();

                let fraud_classification = match task_store.get::<ResponseResult>(&get_key_for_fraud_detection(hash),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::FraudClassification(FraudClassification{title, description, fraud_prediction })), timestamp }) => {
                        Some(fraud_prediction)
                    }
                    Err(_) => {None}
                    _ => {None}
                };

                let mut briefings = Vec::new();

                for i in 0..10 {
                    let gpt3_result_briefing = match task_store.get::<ResponseResult>(&get_key_for_gpt3(hash, &format!("briefing{}",i)), &RetrievalMethod::GetOk) {
                        Ok(Maybe { data: Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::ChatCompletionResult(OpenAIGPTChatCompletionResult { result, .. }))), .. }) => {
                            Some(result)
                        }
                        Err(_) => { None }
                        _ => { None }
                    };
                    if i == 0 {
                        let content = if let Some(completion) = gpt3_result_briefing {
                            format!("⚡ AI-Generated Briefing\n\n{}\n\n🅘 Please note this may contain errors or inaccuracies. It is intended to provide a general overview of the proposal, and should not be relied upon as a definitive or comprehensive analysis. Please review the full proposal before making any decisions.",completion.trim())
                        }else{
                            "This feature is currently only available for legitimate governance proposals that are actively being voted on. 🗳️".trim().to_string()
                        };
                        briefings.push(("summary".to_string(),content.to_string()));
                    }
                    else{
                        //briefings.push(("other".to_string(),format!("{}",gpt3_result_briefing.unwrap_or("This feature is currently only available for legitimate governance proposals that are actively being voted on. 🗳️".to_string()).trim())));
                    }
                }

                let data =  ProposalData {
                        proposal_api: format!("https://libreai.de/cosmos-governance-proposals/{}/{}.html",proposal.blockchain_name.to_string().to_lowercase(),proposal.proposal().map(|x| x.proposal_id.to_string()).unwrap_or("??".to_string())),
                        proposal_link: proposal.governance_proposal_link(),
                        proposal_clickbait: proposal.proposal_clickbait(fraud_classification),
                        proposal_gpt_completions: briefings,
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
                        proposal_title: proposal.get_title_and_description().0,
                        proposal_description: proposal.get_title_and_description().1,
                        proposal_vetoed: proposal.proposal().map(|x| x.final_tally_result.map(|y| y.no_with_veto.parse::<f64>().unwrap_or(0f64) > y.yes.parse::<f64>().unwrap_or(0f64) && y.no_with_veto.parse::<f64>().unwrap_or(0f64) > y.no.parse::<f64>().unwrap_or(0f64))).flatten().unwrap_or(false),
                        proposal_in_deposit_period: proposal.status == ProposalStatus::StatusDepositPeriod,
                        fraud_risk: fraud_classification.unwrap_or(0.0).to_string(),
                        proposal_status_icon: proposal.status.to_icon(),

                };

                if fraud_classification.is_some() || (proposal.status!=ProposalStatus::StatusVotingPeriod && proposal.status!=ProposalStatus::StatusDepositPeriod) {
                    view.push(
                        CosmosRustBotValue::Entry(Entry::Value(Value {
                            timestamp: timestamp.to_owned(),
                            origin: origin.to_owned(),
                            custom_data: CustomData::ProposalData(data),
                            imperative: if list_proposal_hash.contains(&proposal_hash){
                                            ValueImperative::Update
                                        }else{
                                            list_proposal_hash.push(proposal_hash);
                                            ValueImperative::Notify
                                        }
                        })));
                }

                // proposals_for_csv.push(data);
            }
        }
    }

    let result: Maybe<ResponseResult> = Maybe {
        data: Ok(ResponseResult::ProposalDataResult(ProposalDataResult {
            list_proposal_hash
        })),
        timestamp: Utc::now().timestamp(),
    };
    task_store.push(PROPOSAL_DATA_RESULT, result).ok();
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