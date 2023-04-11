use cosmos_rust_package::chrono::Utc;
use cosmos_rust_package::api::custom::query::gov::{get_params, get_proposals, get_tally};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use cosmos_rust_package::api::custom::types::gov::proposal_ext::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::Maybe;
use crate::utils::response::{BlockchainQuery, ResponseResult, TaskResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use cosmos_rust_package::tokio::time::{Instant, sleep, sleep_until};


const TALLY_RESULT_PREFIX: &str = "TALLY_RESULT";
const PARAMS_PREFIX: &str = "PARAMS";

pub fn get_key_for_tally_result(hash: u64) -> String {
    format!("{}_{}",TALLY_RESULT_PREFIX,hash)
}

pub fn get_key_for_params(blockchain_name: &str, params_type: &str) -> String {
    format!("{}_{}_{}",PARAMS_PREFIX,blockchain_name,params_type)
}

fn hash_vec_u8(vec: &Vec<u8>) -> u64 {
    let mut hasher = DefaultHasher::new();
    vec.hash(&mut hasher);
    hasher.finish()
}

// TODO: WARNING: if the page count were to decrease for some reason, database will have orphan entries!
pub async fn fetch_proposals(blockchain: SupportedBlockchain,status: ProposalStatus,task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let continue_at_key = format!("fetch_proposals_for_{}",key);

    let mut keys: Vec<String> = Vec::new();

    let mut next_key = match task_store.get::<ResponseResult>(&continue_at_key,&RetrievalMethod::Get) {
        Ok(item) => {
            match item {
                Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::ContinueAtKey(key))), .. } => {
                    key
                },
                _ => {None}
            }
        },
        Err(_) => {None}
    };

    loop {

        let instance = Instant::now();
        let proposals = get_proposals(blockchain.clone(), status.clone(), next_key.clone()).await;

        // might return unavailable due to rate-limiting policy
        // which makes starting at the beginning over and over inefficient
        // therefore saving continue key
        let item = match proposals {
            Ok(_) => {
                // reset continue key
                Maybe{ data: Ok(ResponseResult::Blockchain(BlockchainQuery::ContinueAtKey(None))), timestamp: Utc::now().timestamp() }
            },
            Err(_) => {
                // save continue key.
                Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::ContinueAtKey(next_key.clone()))), timestamp: Utc::now().timestamp() }
            }
        };
        task_store.push(&continue_at_key,item)?;
        let proposals = proposals?;

        let key1 = format!("page_key_{}_{}", next_key.map(|x| hash_vec_u8(&x)).unwrap_or(0) , key);

        let result: Maybe<ResponseResult> = Maybe {
            data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(proposals.1))),
            timestamp: Utc::now().timestamp(),
        };
        task_store.push(&key1, result)?;

        next_key = proposals.0.clone();

        keys.push(key1);

        if let Some(ref new_next_key) = next_key {
            if new_next_key.is_empty() { // vec![]
                break;
            }
            // continue with valid pagination response for next key.
        }else{ // no pagination response | no next key
            break;
        }

        // rate-limiting
        sleep_until(instance + Duration::from_secs(10)).await;
    }

    Ok(TaskResult{ list_of_keys_modified: keys })
}

pub async fn fetch_tally_results(blockchain: SupportedBlockchain, status: ProposalStatus, task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let continue_at_key = format!("fetch_tally_results_for_{}",key);

    let next_index = match task_store.get::<ResponseResult>(&continue_at_key,&RetrievalMethod::Get) {
        Ok(item) => {
            match item {
                Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::ContinueAtIndex(index))), .. } => {
                    index
                },
                _ => {None}
            }
        },
        Err(_) => {None}
    };

    let mut keys: Vec<String> = Vec::new();

    let mut values: Vec<ProposalExt> = Vec::new();

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {
                for each in proposals.into_iter().filter(|x| x.status == status && x.blockchain.name == blockchain.name && (next_index.is_none() || x.get_proposal_id() >= next_index.unwrap())) {
                    values.push(each);
                }
            }
            _ => {}
        }
    }
    values.sort_by_key(|k| k.get_proposal_id());

    for mut each in values {
        let id = each.get_proposal_id();
        let instance = Instant::now();
        let tally = get_tally(blockchain.clone(), id).await;
        let item = match tally {
            Ok(_) => {
                // reset continue key
                Maybe{ data: Ok(ResponseResult::Blockchain(BlockchainQuery::ContinueAtIndex(None))), timestamp: Utc::now().timestamp() }
            },
            Err(_) => {
                // save continue key.
                Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::ContinueAtIndex(Some(id)))), timestamp: Utc::now().timestamp() }
            }
        };
        task_store.push(&continue_at_key,item)?;
        let tally = tally?;

        let key1 = get_key_for_tally_result(each.object_to_hash());


        let result: Maybe<ResponseResult> = Maybe {
            data: Ok(ResponseResult::Blockchain(BlockchainQuery::TallyResult(tally))),
            timestamp: Utc::now().timestamp(),
        };
        task_store.push(&key1, result)?;
        keys.push(key1);

        // rate-limiting
        sleep_until(instance + Duration::from_secs(10)).await;

    }

    Ok(TaskResult{ list_of_keys_modified: keys })
}

pub async fn fetch_params(blockchain: SupportedBlockchain, params_type: String, task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let params = get_params(blockchain.clone(),params_type.clone()).await?;

    let result: Maybe<ResponseResult> = Maybe {
        data: Ok(ResponseResult::Blockchain(BlockchainQuery::Params(params))),
        timestamp: Utc::now().timestamp(),
    };
    let key1 = get_key_for_params(&blockchain.name,&params_type);
    task_store.push(&key1, result)?;

    Ok(TaskResult{ list_of_keys_modified: vec![key1] })
}