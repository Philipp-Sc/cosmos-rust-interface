use std::collections::HashMap;
use cosmos_rust_package::api::custom::query::gov::ProposalExt;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize,Serialize};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;


#[derive(Debug, Clone, EnumAsInner)]
pub enum ResponseResult {
    ChainRegistry(HashMap<String,SupportedBlockchain>),
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
