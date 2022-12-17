use chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, GPT3Result, GPT3ResultStatus, TaskResult, FraudClassification};
use rust_openai_gpt_tools_socket_ipc::ipc::client_send_openai_gpt_summarization_request;
use rust_openai_gpt_tools_socket_ipc::ipc::OpenAIGPTSummarizationResult;
use crate::services::fraud_detection::FRAUD_DETECTION_PREFIX;


pub const GPT3_PREFIX: &str = "GPT3";

pub async fn gpt3(task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut counter_results = 0usize;
    let mut counter_existing_results = 0usize;

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {

                for each in proposals.iter_mut().filter(|x| x.status==ProposalStatus::StatusVotingPeriod) {

                    let hash = each.title_and_description_to_hash();

                    if task_store.contains_key(&format!("{}_{}",FRAUD_DETECTION_PREFIX,hash)){

                        let fraud_classification = match task_store.get::<ResponseResult>(&format!("{}_{}",FRAUD_DETECTION_PREFIX,hash),&RetrievalMethod::GetOk){
                            Ok(Maybe { data: Ok(ResponseResult::FraudClassification(FraudClassification{title, description, text, fraud_prediction })), timestamp }) => {
                                Some(fraud_prediction)
                            }
                            Err(_) => {None}
                            _ => {None}
                        };
                        if let Some(val) = fraud_classification {
                            if val < 0.7 {

                                if !task_store.contains_key(&format!("{}_{}",GPT3_PREFIX,hash)){ // TODO: need to check if OK or ERROR

                                    let (title,description) = each.get_title_and_description();
                                    let text =  format!("{}/n{}",title,description);
                                    let prompt = "A concise briefing on this governance proposal. Tweet.".to_string();

                                    error!("client_send_openai_gpt_summarization_request");
                                    let result: anyhow::Result<OpenAIGPTSummarizationResult> = client_send_openai_gpt_summarization_request("./tmp/rust_openai_gpt_tools_socket",text,prompt);
                                    error!("OpenAIGPTSummarizationResult: {:?}",result);

                                    let result: Maybe<ResponseResult> = Maybe {
                                        data: match result {
                                                    Ok(data) => Ok(ResponseResult::GPT3Result(GPT3Result{
                                                        text: data.request.text,
                                                        prompt: data.request.prompt,
                                                        result: data.result
                                                    })),
                                                    Err(err) => Err(MaybeError::AnyhowError(err.to_string())),
                                                },
                                        timestamp: Utc::now().timestamp(),
                                    };
                                    keys.push(format!("{}_{}",GPT3_PREFIX,hash));
                                    task_store.push(&keys.last().unwrap(),result).ok();

                                    // progress
                                    let result: Maybe<ResponseResult> = Maybe {
                                        data: Ok(ResponseResult::GPT3ResultStatus(GPT3ResultStatus{
                                            number_of_results: counter_results + counter_existing_results,
                                        })),
                                        timestamp: Utc::now().timestamp(),
                                    };
                                    error!("RustBertGPT3Progress: {:?}",result);

                                    keys.push(key.to_owned());
                                    task_store.push(&key,result).ok();

                                    counter_results+=1usize;
                                }else{
                                    counter_existing_results+=1usize;
                                }
                            }
                        }

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