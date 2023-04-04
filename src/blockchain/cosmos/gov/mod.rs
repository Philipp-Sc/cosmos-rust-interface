use cosmos_rust_package::chrono::Utc;
use cosmos_rust_package::api::custom::query::gov::{get_params, get_proposals, get_tally};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use cosmos_rust_package::api::custom::types::gov::proposal_ext::{ProposalExt, ProposalStatus};
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::Maybe;
use crate::utils::response::{BlockchainQuery, ResponseResult, TaskResult};


const TALLY_RESULT_PREFIX: &str = "TALLY_RESULT";
const PARAMS_PREFIX: &str = "PARAMS";

pub fn get_key_for_tally_result(hash: u64) -> String {
    format!("{}_{}",TALLY_RESULT_PREFIX,hash)
}

pub fn get_key_for_params(blockchain_name: String, params_type: String) -> String {
    format!("{}_{}_{}",PARAMS_PREFIX,blockchain_name,params_type)
}


// TODO: WARNING: if the page count were to decrease for some reason, database will have orphan entries!
pub async fn fetch_proposals(blockchain: SupportedBlockchain,status: ProposalStatus,task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut next_key = None;
    let mut count = 1usize;

    loop {

        let key1 = format!("page_{}_{}", count, key);

        let res = get_proposals(blockchain.clone(), status.clone(), next_key.clone()).await?;

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

pub async fn fetch_tally_results(blockchain: SupportedBlockchain, status: ProposalStatus, task_store: TaskMemoryStore, _key: String) -> anyhow::Result<TaskResult> {

    let mut keys: Vec<String> = Vec::new();

    let mut values: Vec<ProposalExt> = Vec::new();

    for (_val_key, val) in task_store.value_iter::<ResponseResult>(&RetrievalMethod::GetOk) {
        match val {
            Maybe { data: Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(mut proposals))), timestamp } => {
                for each in proposals.into_iter().filter(|x| x.status == status && x.blockchain_name == blockchain.name) {
                    values.push(each);
                }
            }
            _ => {}
        }
    }
    for mut each in values {
        let id = each.get_proposal_id();
        let tally = get_tally(blockchain.clone(), id).await?;

        let key1 = get_key_for_tally_result(each.to_hash());


        let result: Maybe<ResponseResult> = Maybe {
            data: Ok(ResponseResult::Blockchain(BlockchainQuery::TallyResult(tally))),
            timestamp: Utc::now().timestamp(),
        };
        task_store.push(&key1, result)?;
        keys.push(key1);
    }

    Ok(TaskResult{ list_of_keys_modified: keys })
}

pub async fn fetch_params(blockchain: SupportedBlockchain, params_type: String, task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {

    let params = get_params(blockchain.clone(),params_type.clone()).await?;

    let result: Maybe<ResponseResult> = Maybe {
        data: Ok(ResponseResult::Blockchain(BlockchainQuery::Params(params))),
        timestamp: Utc::now().timestamp(),
    };
    let key1 = get_key_for_params(blockchain.name,params_type);
    task_store.push(&key1, result)?;

    Ok(TaskResult{ list_of_keys_modified: vec![key1] })
}