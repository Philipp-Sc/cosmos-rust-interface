use cosmos_rust_package::chrono::Utc;
use cosmos_rust_package::api::core::cosmos::channels;
use crate::utils::entry::db::TaskMemoryStore;
use crate::utils::entry::Maybe;
use crate::utils::response::{ResponseResult, TaskResult};

pub async fn get_supported_blockchains_from_chain_registry(path: String, task_store: TaskMemoryStore, key: String) -> anyhow::Result<TaskResult> {
    let res = channels::get_supported_blockchains_from_chain_registry(path,true,None).await;

    let result: Maybe<ResponseResult> = Maybe {
        data: Ok(ResponseResult::ChainRegistry(res)),
        timestamp: Utc::now().timestamp(),
    };
    task_store.push("internal_chain_registry",result)?;
    Ok(TaskResult{ list_of_keys_modified: vec!["internal_chain_registry".to_string()] })
}
