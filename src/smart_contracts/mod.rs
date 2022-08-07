use serde::Deserialize;
use serde::Serialize;
use enum_as_inner::EnumAsInner;

// query for specific smart contract information
pub mod cosmos;
pub mod terra;

#[derive(Debug, Clone, Serialize, Deserialize, EnumAsInner)]
pub enum SmartContractsQuery {
    None,
    Error,
}