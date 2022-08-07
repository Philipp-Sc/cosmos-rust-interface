use cosmos_rust_package::api::custom::query::gov::{get_proposals as get_gov_proposals, ProposalStatus};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use crate::blockchain::BlockchainQuery;
use crate::ResponseResult;

pub async fn get_proposals(blockchain: SupportedBlockchain,status: ProposalStatus) -> anyhow::Result<ResponseResult> {
    let res = get_gov_proposals(blockchain, status).await?;
    Ok(ResponseResult::Blockchain(BlockchainQuery::GovProposals(res)))
}