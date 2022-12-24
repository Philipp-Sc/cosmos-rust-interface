use chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, FraudClassification, FraudClassificationStatus, TaskResult};
use rust_bert_fraud_detection_socket_ipc::ipc::client_send_rust_bert_fraud_detection_request;
use rust_bert_fraud_detection_socket_ipc::ipc::RustBertFraudDetectionResult;


const FRAUD_DETECTION_PREFIX: &str = "FRAUD_DETECTION";


pub fn get_key_for_fraud_detection(hash: u64) -> String {
    format!("{}_{}",FRAUD_DETECTION_PREFIX,hash)
}

// TODO: potentially batch multiple requests.
pub async fn fraud_detection(task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut counter_classifications = 0usize;
    let mut counter_existing_classifications = 0usize;

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {

                for each in proposals.iter_mut().filter(|x| x.status==ProposalStatus::StatusDepositPeriod || x.status==ProposalStatus::StatusVotingPeriod) {

                    let hash = each.title_and_description_to_hash();
                    let key_for_hash = get_key_for_fraud_detection(hash);

                    if !task_store.contains_key(&key_for_hash){ // TODO: need to check if OK or ERROR

                        let text =  each.proposal_details(None);

                        info!("client_send_rust_bert_fraud_detection_request");
                        let result: anyhow::Result<RustBertFraudDetectionResult> = client_send_rust_bert_fraud_detection_request("./tmp/rust_bert_fraud_detection_socket",vec![text.clone()]);
                        info!("RustBertFraudDetectionResult: {:?}",result);

                        let (title, description) = each.get_title_and_description();
                        let result: Maybe<ResponseResult> = Maybe {
                            data: match result {
                                Ok(data) => Ok(ResponseResult::FraudClassification(FraudClassification{
                                    title,
                                    description,
                                    text,
                                    fraud_prediction: data.fraud_probabilities[0]
                                })),
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
                        error!("RustBertFraudDetectionProgress: {:?}",result);

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