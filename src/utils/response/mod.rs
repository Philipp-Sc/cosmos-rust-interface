
use cosmos_rust_package::api::custom::query::gov::ProposalExt;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize,Serialize};


#[derive(Debug, Clone, EnumAsInner)]
pub enum ResponseResult {
    Blockchain(BlockchainQuery),
    Services(ServicesQuery),
    SmartContracts(SmartContractsQuery),
    LogEntry(String),  // used for logging
    Text(String),  // used for logs
}

#[derive(Debug, Clone, EnumAsInner)]
pub enum BlockchainQuery {
    GovProposals(Vec<ProposalExt>),
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum ServicesQuery {
    None,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum SmartContractsQuery {
    None,
    Error,
}
