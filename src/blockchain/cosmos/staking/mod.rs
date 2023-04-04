use cosmos_rust_package::chrono::Utc;
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use cosmos_rust_package::api::custom::query::staking::get_pool;
use crate::utils::entry::db::{RetrievalMethod, TaskMemoryStore};
use crate::utils::entry::Maybe;
use crate::utils::response::{BlockchainQuery, ResponseResult, TaskResult};


const POOL_PREFIX: &str = "POOL";

pub fn get_key_for_pool(blockchain_name: &str) -> String {
    format!("{}_{}",POOL_PREFIX,blockchain_name)
}

pub async fn fetch_pool(blockchain: SupportedBlockchain, task_store: TaskMemoryStore, _key: String) -> anyhow::Result<TaskResult> {

    let pool = get_pool(blockchain.clone()).await?;

    let result: Maybe<ResponseResult> = Maybe {
        data: Ok(ResponseResult::Blockchain(BlockchainQuery::Pool(pool))),
        timestamp: Utc::now().timestamp(),
    };
    let key1 = get_key_for_pool(&blockchain.name);
    task_store.push(&key1, result)?;

    Ok(TaskResult{ list_of_keys_modified: vec![key1] })
}