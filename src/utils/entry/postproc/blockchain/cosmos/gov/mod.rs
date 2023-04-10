use std::collections::HashMap;
use cosmos_rust_package::chrono::Utc;
use crate::utils::entry::*;
use strum::IntoEnumIterator;
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::response::{ResponseResult, BlockchainQuery, FraudClassification, ProposalDataResult};

use serde::{Deserialize,Serialize};
use cosmos_rust_package::api::custom::types::gov::proposal_ext::{ProposalExt, ProposalStatus, ProposalTime};
use rust_openai_gpt_tools_socket_ipc::ipc::{OpenAIGPTResult, OpenAIGPTChatCompletionResult};
use crate::blockchain::cosmos::gov::{get_key_for_params, get_key_for_tally_result};
use crate::blockchain::cosmos::staking::get_key_for_pool;
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
/// Augmented with:
/// - Fraud Detection
/// - GPT3 Briefing
/// - Tally Result
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

                let hash = proposal.object_to_hash();

                let tally_result = match task_store.get::<ResponseResult>(&get_key_for_tally_result(hash),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::TallyResult(tally_result))), timestamp }) => {
                        Some(tally_result)
                    }
                    Err(_) => {None}
                    _ => {None}
                };

                let blockchain_pool = match task_store.get::<ResponseResult>(&get_key_for_pool(&proposal.blockchain.name),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::Pool(pool))), timestamp }) => {
                        Some(pool)
                    }
                    Err(_) => {None}
                    _ => {None}
                };

                let deposit_param = match task_store.get::<ResponseResult>(&get_key_for_params(&proposal.blockchain.name,"deposit"),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::Params(params))), timestamp }) => {
                        Some(params)
                    }
                    Err(_) => {None}
                    _ => {None}
                };
                let voting_param = match task_store.get::<ResponseResult>(&get_key_for_params(&proposal.blockchain.name,"voting"),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::Params(params))), timestamp }) => {
                        Some(params)
                    }
                    Err(_) => {None}
                    _ => {None}
                };
                let tallying_param = match task_store.get::<ResponseResult>(&get_key_for_params(&proposal.blockchain.name,"tallying"),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::Params(params))), timestamp }) => {
                        Some(params)
                    }
                    Err(_) => {None}
                    _ => {None}
                };

                let fraud_classification = match task_store.get::<ResponseResult>(&get_key_for_fraud_detection(hash),&RetrievalMethod::GetOk){
                    Ok(Maybe { data: Ok(ResponseResult::FraudClassification(FraudClassification{title, description, fraud_prediction })), timestamp }) => {
                        Some(fraud_prediction)
                    }
                    Err(_) => {None}
                    _ => {None}
                };

                let headline1 = "🅘 AI-Generated Overview\n\n";
                let headline2 = "⚡ AI-Generated Briefing\n\n";
                let info = "\n\n🅘 Please note this may contain errors or inaccuracies. It is intended to provide a general overview of the proposal, and should not be relied upon as a definitive or comprehensive analysis. Please review the full proposal before making any decisions.";
                let unavailable = "This feature is currently only available for legitimate governance proposals that are actively being voted on. 🗳️";

                let summary = match task_store.get::<ResponseResult>(&get_key_for_gpt3(hash, &format!("SUMMARY_{}",0)), &RetrievalMethod::GetOk) {
                    Ok(Maybe { data: Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::ChatCompletionResult(OpenAIGPTChatCompletionResult { result, .. }))), .. }) => {
                        format!("{}{}{}",headline1,result.trim(),info)
                    }
                    Err(_) => { unavailable.to_string() }
                    _ => { unavailable.to_string() }
                };
                let briefing = match task_store.get::<ResponseResult>(&get_key_for_gpt3(hash, &format!("BRIEFING_{}",0)), &RetrievalMethod::GetOk) {
                    Ok(Maybe { data: Ok(ResponseResult::OpenAIGPTResult(OpenAIGPTResult::ChatCompletionResult(OpenAIGPTChatCompletionResult { result, .. }))), .. }) => {
                        format!("{}{}{}",headline2,result.trim(),info)
                    }
                    Err(_) => { unavailable.to_string() }
                    _ => { unavailable.to_string() }
                };


                let data =  ProposalData::new(
                    &proposal,
                    &fraud_classification,
                    summary,
                    briefing,
                    tally_result,
                    tallying_param,
                    deposit_param,
                    voting_param,
                    blockchain_pool
                    );

                if fraud_classification.is_some() || (proposal.status!=ProposalStatus::StatusVotingPeriod && proposal.status!=ProposalStatus::StatusDepositPeriod) {
                        let id = proposal.status_based_id();

                        view.push(
                        CosmosRustBotValue::Entry(Entry::Value(Value {
                            timestamp: timestamp.to_owned(),
                            origin: origin.to_owned(),
                            custom_data: CustomData::ProposalData(data),
                            // as long as the ProposalStatus stays the same, do not notify any subscription (i.e not send a second identical notification)
                            imperative: if list_proposal_hash.contains(&id){
                                            ValueImperative::Update
                                        }else{
                                            list_proposal_hash.push(id);
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