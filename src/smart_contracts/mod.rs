use serde::Deserialize;
use serde::Serialize;
use enum_as_inner::EnumAsInner;
use cosmos_rust_package::api::custom::query::gov::ProposalExt;

// query for specific smart contract information
pub mod cosmos;
pub mod terra;

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum SmartContractsQuery {
    None,
    Error,
}