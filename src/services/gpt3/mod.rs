use chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::query::gov::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, GPT3Result, GPT3ResultStatus, TaskResult, FraudClassification};
use rust_openai_gpt_tools_socket_ipc::ipc::client_send_openai_gpt_summarization_request;
use rust_openai_gpt_tools_socket_ipc::ipc::OpenAIGPTSummarizationResult;
use crate::services::fraud_detection::get_key_for_hash as fraud_detection_get_key_for_hash;


const GPT3_PREFIX: &str = "GPT3";

pub fn get_key_for_hash(hash: u64, prompt_id: &str) -> String {
    format!("{}_{}_{}",GPT3_PREFIX, prompt_id, hash)
}

pub async fn gpt3(task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut counter_results = 0usize;
    let mut counter_existing_results = 0usize;

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {
                for each in proposals.iter_mut().filter(|x| x.status == ProposalStatus::StatusVotingPeriod) {
                    let hash = each.title_and_description_to_hash();
                    let fraud_detection_key_for_hash = fraud_detection_get_key_for_hash(hash);

                    if task_store.contains_key(&fraud_detection_key_for_hash) {
                        let fraud_classification = match task_store.get::<ResponseResult>(&fraud_detection_key_for_hash, &RetrievalMethod::GetOk) {
                            Ok(Maybe { data: Ok(ResponseResult::FraudClassification(FraudClassification { title, description, text, fraud_prediction })), timestamp }) => {
                                Some(fraud_prediction)
                            }
                            Err(_) => { None }
                            _ => { None }
                        };
                        if let Some(val) = fraud_classification {
                            if val < 0.7 {
                                let (title, description) = each.get_title_and_description();
                                let text = format!("{}/n{}", title, description);
                                let prompts = [
                                    "Provide a brief overview of the motivation or purpose behind this governance proposal. Tweet.",
                                    "Bullet points: Benefits, Risks, Recommendations or advice for evaluating the proposal."
                                ];
                                let completion_token_limits = [
                                    100u16,
                                    1000u16,
                                ];

                                for i in 0..prompts.len() {
                                    if let Some(inserted_key) = insert_gpt3_result(&task_store, hash, &format!("briefing{}",i), &text, prompts[i],completion_token_limits[i]) {
                                        counter_results += 1usize;

                                        // progress
                                        let result: Maybe<ResponseResult> = Maybe {
                                            data: Ok(ResponseResult::GPT3ResultStatus(GPT3ResultStatus {
                                                number_of_results: counter_results + counter_existing_results,
                                            })),
                                            timestamp: Utc::now().timestamp(),
                                        };
                                        error!("RustBertGPT3Progress: {:?}",result);

                                        keys.push(key.to_owned());
                                        task_store.push(&key, result).ok();
                                    } else {
                                        counter_existing_results += 1usize;
                                    }
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

pub fn insert_gpt3_result(task_store: &TaskMemoryStore, hash: u64, prompt_id: &str, text: &str, prompt: &str, completion_token_limit: u16) -> Option<String> {


    let key_for_hash = get_key_for_hash(hash,prompt_id);

    if !task_store.contains_key(&key_for_hash) {


        error!("client_send_openai_gpt_summarization_request");
        let result: anyhow::Result<OpenAIGPTSummarizationResult> = client_send_openai_gpt_summarization_request("./tmp/rust_openai_gpt_tools_socket", text.to_owned(), prompt.to_owned(),completion_token_limit);
        error!("OpenAIGPTSummarizationResult: {:?}",result);

        let result: Maybe<ResponseResult> = Maybe {
            data: match result {
                Ok(data) => Ok(ResponseResult::GPT3Result(GPT3Result {
                    text: data.request.text,
                    prompt: data.request.prompt,
                    result: data.result
                })),
                Err(err) => Err(MaybeError::AnyhowError(err.to_string())),
            },
            timestamp: Utc::now().timestamp(),
        };
        task_store.push(&key_for_hash, result).ok();
        Some(key_for_hash)
    }else{
        None
    }
}