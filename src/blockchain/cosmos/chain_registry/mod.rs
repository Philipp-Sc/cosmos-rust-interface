
use cosmos_rust_package::api::core::cosmos::channels;
use crate::utils::response::{ResponseResult};

pub async fn get_supported_blockchains_from_chain_registry(path: String) -> anyhow::Result<ResponseResult> {
    let res = channels::get_supported_blockchains_from_chain_registry(path,true,None).await;
    Ok(ResponseResult::ChainRegistry(res))
}
