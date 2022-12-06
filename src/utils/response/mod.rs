use std::collections::HashMap;
use cosmos_rust_package::api::custom::query::gov::ProposalExt;
use enum_as_inner::EnumAsInner;
use serde::{Deserialize,Serialize};
use cosmos_rust_package::api::core::cosmos::channels::SupportedBlockchain;


#[derive(Serialize,Deserialize,Debug, Clone, EnumAsInner)]
pub enum ResponseResult {
    ChainRegistry(HashMap<String,SupportedBlockchain>),
    Blockchain(BlockchainQuery),
    Services(ServicesQuery),
    SmartContracts(SmartContractsQuery),
    FraudClassification(FraudClassification),
    FraudClassificationStatus(FraudClassificationStatus),
    TaskResult(TaskResult),
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct TaskResult {
    pub list_of_keys_modified: Vec<String>,
}

/*
impl From<Vec<u8>> for ResponseResult {
    fn from(item: Vec<u8>) -> Self  {
        bincode::deserialize(&item[..]).unwrap()
    }
}
impl From<ResponseResult> for Vec<u8> {
    fn from(item: ResponseResult) -> Self  {
        bincode::serialize(&item).unwrap()
    }
}*/
/*
impl TryFrom<Vec<u8>> for ResponseResult {
    type Error = anyhow::Error;
    fn try_from(item: Vec<u8>) -> anyhow::Result<Self>  {
        Ok(bincode::deserialize(&item[..])?)
    }
}
impl TryFrom<ResponseResult> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(item: ResponseResult) -> anyhow::Result<Self>  {
        Ok(bincode::serialize(&item)?)
    }
}*/

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct FraudClassificationStatus {
    pub number_of_classifications: usize,
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct FraudClassification {
    pub title: String,
    pub description: String,
    pub text: String,
    pub fraud_prediction: f64,
}

#[derive(Serialize,Deserialize,Debug, Clone, EnumAsInner)]
pub enum BlockchainQuery {
    //NextKey(Option<Vec<u8>>),
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
