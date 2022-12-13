use chrono::Utc;
use cosmos_rust_package::api::custom::query::gov::{get_proposals as get_gov_proposals, ProposalStatus};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::Maybe;
use crate::utils::response::{BlockchainQuery, ResponseResult, TaskResult};


// TODO: WARNING: if the page count were to decrease for some reason, database will have orphan entries!
pub async fn get_proposals(blockchain: SupportedBlockchain,status: ProposalStatus,task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut next_key = None;
    let mut count = 1usize;

    loop {

        let key1 = format!("page_{}_{}", count, key);

        let res = get_gov_proposals(blockchain.clone(), status.clone(), next_key.clone()).await?;

        let result: Maybe<ResponseResult> = Maybe {
            data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(res.1))),
            timestamp: Utc::now().timestamp(),
        };
        task_store.push(&key1, result)?;

        next_key = res.0.clone();

        keys.push(key1);

        if let Some(ref new_next_key) = next_key {
            if new_next_key.is_empty() { // vec![]
                break;
            }
            // continue with valid pagination response for next key.
        }else{ // no pagination response | no next key
            break;
        }
        count += 1;
    }

    Ok(TaskResult{ list_of_keys_modified: keys })
}