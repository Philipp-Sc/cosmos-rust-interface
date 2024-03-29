use cosmos_rust_package::chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::types::gov::proposal_ext::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, FraudClassification, FraudClassificationStatus, TaskResult};
use rust_bert_fraud_detection_socket_ipc::ipc::client_send_rust_bert_fraud_detection_request;
use rust_bert_fraud_detection_socket_ipc::ipc::RustBertFraudDetectionResult;

use csv::Writer;
use crate::blockchain::cosmos::gov::get_key_for_tally_result;

const FRAUD_DETECTION_PREFIX: &str = "FRAUD_DETECTION";


pub fn get_key_for_fraud_detection(hash: u64) -> String {
    format!("{}_{}",FRAUD_DETECTION_PREFIX,hash)
}

// TODO: potentially batch multiple requests.

pub async fn fraud_detection(task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {


    let mut wtr = csv::Writer::from_path("./tmp/governance_proposal_spam_likelihood.csv").unwrap();
    wtr.write_record(&["body","label"]).unwrap();

    let mut keys: Vec<String> = Vec::new();

    let mut counter_classifications = 0usize;
    let mut counter_existing_classifications = 0usize;

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {

                for each in proposals.iter_mut().filter(|x| x.status!=ProposalStatus::StatusDepositPeriod && x.status!=ProposalStatus::StatusVotingPeriod) {


                    let title = each.get_title();
                    let description = each.get_description();

                    let text =  format!("{}\n\n{}",title,description);
                    let spam_likelihood = each.spam_likelihood();

                    if let Some(value) = spam_likelihood {
                        wtr.write_record(&[text.as_str(), value.to_string().as_str()]).unwrap();
                    }

                }
                wtr.flush().unwrap();


                for each in proposals.iter_mut().filter(|x| x.status==ProposalStatus::StatusDepositPeriod || x.status==ProposalStatus::StatusVotingPeriod) {

                    let hash = each.object_to_hash();
                    let key_for_hash = get_key_for_fraud_detection(hash);

                    if !task_store.contains_key(&key_for_hash){ // TODO: need to check if OK or ERROR


                        let title = each.get_title();
                        let description = each.get_description();
                        let text =  format!("{}\n\n{}",&title,&description);

                        info!("client_send_rust_bert_fraud_detection_request");
                        let result: anyhow::Result<RustBertFraudDetectionResult> = client_send_rust_bert_fraud_detection_request("./tmp/rust_bert_fraud_detection_socket",vec![text.clone()]);
                        info!("RustBertFraudDetectionResult: {:?}",result);


                        let result: Maybe<ResponseResult> = Maybe {
                            data: match result {
                                Ok(data) => {

                                    let fraud_classification = FraudClassification {
                                        title,
                                        description,
                                        fraud_prediction: data.fraud_probabilities[0]
                                    };

                                    Ok(ResponseResult::FraudClassification(fraud_classification))

                                } ,
                                Err(err) => Err(MaybeError::AnyhowError(err.to_string())),
                            },
                            timestamp: Utc::now().timestamp(),
                        };

                        keys.push(key_for_hash);
                        task_store.push(&keys.last().unwrap(),result).ok();

                        // progress
                        let result: Maybe<ResponseResult> = Maybe {
                            data: Ok(ResponseResult::FraudClassificationStatus(FraudClassificationStatus{
                                number_of_classifications: counter_classifications + counter_existing_classifications,
                            })),
                            timestamp: Utc::now().timestamp(),
                        };
                        info!("RustBertFraudDetectionProgress: {:?}",result);

                        keys.push(key.to_owned());
                        task_store.push(&key,result).ok();

                        counter_classifications+=1usize;
                    }else{
                        counter_existing_classifications+=1usize;
                    }
                }
            },
            _ => {}
        }
    }
    Ok(TaskResult{
        list_of_keys_modified: keys
    })
}


pub fn validate_fraud_detection_result(task_store: &TaskMemoryStore, hash: u64) -> bool {

    match task_store.get::<ResponseResult>(&get_key_for_tally_result(hash),&RetrievalMethod::GetOk){
        Ok(Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::TallyResult(tally_result))), timestamp }) => {
            if let Some(spam_likelihood) = tally_result.spam_likelihood() {
                if spam_likelihood >=0.5 {
                    return false;
                }
            }
        }
        _ => {}
    };

    let fraud_detection_key_for_hash = get_key_for_fraud_detection(hash);

    if task_store.contains_key(&fraud_detection_key_for_hash) {
        match task_store.get::<ResponseResult>(&fraud_detection_key_for_hash, &RetrievalMethod::GetOk) {
            Ok(Maybe { data: Ok(ResponseResult::FraudClassification(FraudClassification { fraud_prediction, .. })), .. }) => {
                if fraud_prediction < 0.7 {
                    return true;
                }else {
                    return false;
                }
            }
            Err(_) => { return false; }
            _ => { return false; }
        }
    }
    return false;
}