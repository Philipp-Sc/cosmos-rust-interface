use cosmos_rust_package::api::core::cosmos;
use cosmos_rust_package::api::custom::query::gov::{get_proposals, ProposalStatus};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;
use crate::blockchain::BlockchainQuery;

pub async fn governance() -> anyhow::Result<BlockchainQuery> {
    let res = get_proposals(SupportedBlockchain::Terra, ProposalStatus::StatusPassed).await?;
    Ok(BlockchainQuery::GovProposals(res))
}