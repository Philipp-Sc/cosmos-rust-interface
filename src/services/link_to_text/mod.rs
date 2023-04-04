use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::ptr::hash;
use cosmos_rust_package::chrono::Utc;
use log::{debug, error, info};
use cosmos_rust_package::api::custom::types::gov::proposal_ext::{ProposalExt, ProposalStatus};
use cosmos_rust_package::api::custom::query::gov::{LINK_FINDER};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::*;
use crate::utils::response::{ResponseResult, BlockchainQuery, LinkToTextResult, LinkToTextResultStatus, TaskResult};
use rust_link_to_text_socket_ipc::ipc::{client_send_link_to_text_request};
use rust_link_to_text_socket_ipc::ipc::LinkToTextResult as LinkToTextResultIPC;


const LINK_TO_TEXT_PREFIX: &str = "LINK_TO_TEXT";

pub fn get_key_for_link_to_text(link_id: &str) -> String {
    format!("{}_{}",LINK_TO_TEXT_PREFIX, link_id)
}

pub fn extract_links(text: &str) -> Vec<String> {

    let links = LINK_FINDER.links(&text);
    let mut output: Vec<String> = Vec::new();
    for link in links  {
        output.push(link.as_str().to_string());
    }
    // Convert the vector to a HashSet
    let set: HashSet<String> = output.into_iter().collect();
    // Convert the HashSet back to a vector
    set.into_iter().collect()
}

pub fn link_to_id(text: &String) -> String {
    let mut s = DefaultHasher::new();
    text.hash(&mut s);
    format!("link{}",s.finish())
}

pub fn string_to_hash(text: &str) -> u64 {
    let mut s = DefaultHasher::new();
    text.hash(&mut s);
    s.finish()
}


pub async fn link_to_text(task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut number_of_new_results = 0usize;
    let mut number_of_stored_results = 0usize;

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {
                for each in proposals.iter_mut().filter(|x| x.status == ProposalStatus::StatusVotingPeriod) {

                    let description = each.get_description();

                    let links = extract_links(&description);

                    for i in 0..links.len()  {

                        let key_for_hash = get_key_for_link_to_text(&link_to_id(&links[i]));
                        let insert_result = insert_link_to_text_result(&task_store, &key_for_hash, &links[i]);
                        insert_progress(&task_store, &key, &mut keys, &mut number_of_new_results, &mut number_of_stored_results, if insert_result {Some(key_for_hash)}else{None});

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

pub fn insert_progress(task_store: &TaskMemoryStore, key: &str, keys: &mut Vec<String>, number_of_new_results: &mut usize, number_of_stored_results: &mut usize, insert_result: Option<String>) {
    if let Some(inserted_key) = insert_result {
        *number_of_new_results += 1usize;

        let progress: Maybe<ResponseResult> = Maybe {
            data: Ok(ResponseResult::LinkToTextResultStatus(LinkToTextResultStatus {
                number_of_results: *number_of_new_results + *number_of_stored_results,
            })),
            timestamp: Utc::now().timestamp(),
        };
        info!("insert_progress: {:?}",progress);

        keys.push(key.to_owned());
        task_store.push(&key, progress).ok();
    } else {
        *number_of_stored_results += 1usize;
    }
}

pub fn insert_link_to_text_result(task_store: &TaskMemoryStore, key: &str, link: &str) -> bool {

    if !task_store.contains_key(&key) {
        info!("Sending request to link-to-text service for key {}", key);
        let result: anyhow::Result<LinkToTextResultIPC> = client_send_link_to_text_request("./tmp/rust_link_to_text_socket", link.to_owned());
        match result {
            Ok(data) => {
                info!("Successfully obtained LinkToTextResult for key {}", key);
                let response_result = Maybe {
                    data: Ok(ResponseResult::LinkToTextResult(LinkToTextResult::new(link, data.text_nodes, data.hierarchical_segmentation, 300))),
                    timestamp: Utc::now().timestamp(),
                };
                if let Err(error) = task_store.push(&key, response_result) {
                    error!("Failed to insert response result for key {}: {:?}", key, error);
                }
                true
            },
            Err(error) => {
                let response_result: Maybe<ResponseResult> = Maybe {
                    data: Err(MaybeError::AnyhowError(error.to_string())),
                    timestamp: Utc::now().timestamp(),
                };
                if let Err(error) = task_store.push(&key, response_result) {
                    error!("Failed to insert response result for key {}: {:?}", key, error);
                }
                error!("Failed to obtain LinkToTextResult for key {}: {:?}", key, error);
                true
            }
        }
    }else{
        info!("Key {} already exists in task store. Skipping insertion", key);
        false
    }
}
