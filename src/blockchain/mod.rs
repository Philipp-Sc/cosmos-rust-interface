use serde::Deserialize;
use serde::Serialize;
use enum_as_inner::EnumAsInner;
use cosmos_rust_package::api::custom::query::gov::ProposalExt;

// to query the blockchain for on-chain information
// query blocks, block height, meta information, past transactions
pub mod cosmos;

#[derive(Debug, Clone, EnumAsInner)]
pub enum BlockchainQuery {
    GovProposals(Vec<ProposalExt>),
    Json(serde_json::Value),
    Text(String),
}